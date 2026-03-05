#![cfg(feature = "bevy_client")]

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bevy::app::AppExit;
use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::hierarchy::ChildBuild;
use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::math::{Mat4, Rect};
use bevy::prelude::*;
use bevy::render::camera::{ClearColorConfig, ScalingMode};
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::view::RenderLayers;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use lce_rust::client::asset_pipeline::{RuntimeAssetManifest, stage_default_runtime_assets};
use lce_rust::client::chunk_streaming::{
    chunk_diff, desired_chunk_window, lifecycle_note_chunk_loaded, lifecycle_note_chunk_unloaded,
    parse_boolean_flag, performance_logging_enabled, player_chunk_from_position,
};
use lce_rust::client::clouds::{
    CLOUD_ADVANCED_SECTION_RADIUS, CLOUD_ADVANCED_TEXEL_WORLD_SIZE,
    CLOUD_ADVANCED_TEXELS_PER_SECTION, CLOUD_ALPHA, CLOUD_LAYER_THICKNESS, CLOUD_TEXEL_UV_SCALE,
    cloud_tick_time, cloud_uv_motion, cloud_world_y, clouds_visible_for_camera_block,
};
use lce_rust::client::crafting_ui::{
    collect_crafting_recipe_state, crafting_recipe_count_label, crafting_recipe_title,
};
use lce_rust::client::creative_ui::{
    CREATIVE_SELECTOR_COLUMNS, CREATIVE_SELECTOR_ROWS, CREATIVE_TABS, CreativeInventoryTab,
    creative_next_dynamic_group, creative_selector_entries_page_for_dynamic_group,
    creative_tab_dynamic_group_count, creative_tab_entry_page_count_for_dynamic_group,
    creative_tab_icon_item_id, creative_tab_title, place_creative_entry_in_hotbar,
};
use lce_rust::client::gameplay_ui::{
    allow_cursor_capture, allow_first_person_item_view, allow_first_person_view,
    hide_gameplay_overlay, show_death_screen, show_pause_menu,
};
use lce_rust::client::hotbar_ui::collect_hotbar_state;
use lce_rust::client::interaction::{
    BlockAction, BlockRaycastHit, INTERACTION_DISTANCE_BLOCKS, apply_block_action,
    forward_vector_from_yaw_pitch, movement_axes_from_yaw, placement_intersects_player_collider,
    raycast_first_non_air_block, raycast_first_solid_block, target_chunk_for_block,
};
use lce_rust::client::inventory_ui::collect_inventory_state;
use lce_rust::client::lifecycle_hooks::{
    DEFAULT_BOOT_DAY_TIME, RuntimeEnvironment, align_total_ticks_to_day_time,
    consume_lifecycle_events, sky_brightness_for, sky_color_from_brightness,
};
use lce_rust::client::particles::terrain_break_particle_tile;
use lce_rust::client::terrain_meshing::{
    BlockFace, TERRAIN_ATLAS_TILES, TerrainMeshData, atlas_tile_for_block_face,
    build_block_break_overlay_mesh_data, build_chunk_mesh_data, dirty_chunks_for_block,
};
use lce_rust::client::world_worker::{ChunkDataSource, GeneratedChunk, WorldWorker};
use lce_rust::save::world_io::{load_world_snapshot, save_world_snapshot};
use lce_rust::world::{
    BlockPos, BlockWorld, ChunkLifecycleController, ChunkPos, DAY_LENGTH_TICKS, HOTBAR_SLOTS,
    INVENTORY_SLOTS, ItemStack, LAVA_SOURCE_BLOCK_ID, LEVER_BLOCK_ID, MovementInput,
    OfflineGameSession, RECIPES, REDSTONE_TORCH_OFF_BLOCK_ID, REDSTONE_TORCH_ON_BLOCK_ID,
    ScheduledTick, WALK_SPEED_BLOCKS_PER_SECOND, WATER_FLOWING_BLOCK_ID, WATER_SOURCE_BLOCK_ID,
    WeatherKind, WorldSession, block_id_for_item, craft_recipe, fluid_ticks_for_block_change,
    is_fluid_block, is_solid_block_for_player_collision, process_scheduled_fluid_tick,
    process_scheduled_redstone_tick, recipe_by_id, redstone_ticks_for_block_change,
};

const DEFAULT_SAVE_ROOT_PATH: &str = "saves/dev_world_v2";
const CAMERA_EYE_HEIGHT: f32 = 1.62;
const DEFAULT_FOV_DEGREES: f32 = 110.0;
const LOOK_SENSITIVITY_RADIANS_PER_PIXEL: f32 = 0.003;
const MAX_PITCH_RADIANS: f32 = 1.45;
const CHUNK_LOAD_RADIUS: i32 = 2;
const MAX_PENDING_CHUNK_REQUESTS: usize = 4;
const MAX_CHUNK_REQUESTS_PER_FRAME: usize = 2;
const MAX_GENERATED_CHUNKS_APPLIED_PER_FRAME: usize = 1;
const HOTBAR_GUI_SCALE: f32 = 2.0;
const INVENTORY_GUI_SCALE: f32 = 3.0;
const LEGACY_MENU_GUI_SCALE: f32 = 2.0;
const LEGACY_MENU_BUTTON_WIDTH_PERCENT: f32 = 35.3;
const LEGACY_MENU_BUTTON_HEIGHT_PERCENT: f32 = 4.55;
const LEGACY_MENU_BUTTON_STEP_PERCENT: f32 = 7.12;
const LEGACY_MENU_LOGO_TOP_PERCENT: f32 = 8.5;
const LEGACY_MENU_LOGO_FIRST_LEFT_PERCENT: f32 = 32.5;
const LEGACY_MENU_LOGO_SECOND_LEFT_PERCENT: f32 = 52.3;
const LEGACY_MENU_LOGO_PART_WIDTH_PERCENT: f32 = 19.8;
const LEGACY_MENU_LOGO_PART_HEIGHT_PERCENT: f32 = 8.7;
const LEGACY_MENU_BUTTONS_TOP_PERCENT: f32 = 42.5;
const HOTBAR_BOTTOM_OFFSET: f32 = 8.0;
const CREATIVE_PANEL_WIDTH: f32 = 196.0;
const CREATIVE_PANEL_HEIGHT: f32 = 170.0;
const UI_ITEM_RENDER_LAYER: usize = 1;
const FIRST_PERSON_ITEM_RENDER_LAYER: usize = 2;
const FIRST_PERSON_ITEM_FOV_DEGREES: f32 = 70.0;
const UI_ITEM_MODEL_SCALE: f32 = 0.55;
const HEART_ICON_SIZE: f32 = 9.0;
const HEART_ICON_STRIDE: f32 = 8.0;
const HUD_STATUS_ROW_WIDTH: f32 = HEART_ICON_SIZE + HEART_ICON_STRIDE * 9.0;
const HUD_XP_BAR_WIDTH: f32 = 182.0;
const HUD_XP_BAR_HEIGHT: f32 = 5.0;
const HUD_XP_BAR_TO_HOTBAR_GAP: f32 = 3.0;
const HUD_STATUS_TO_XP_GAP: f32 = 1.0;
const HUD_FOOD_UV_Y: f32 = 27.0;
const HUD_XP_UV_BG_Y: f32 = 64.0;
const HUD_XP_UV_FILL_Y: f32 = 69.0;
const SPRINT_RUN_THRESHOLD: f32 = 0.8;
const SPRINT_TRIGGER_WINDOW_TICKS: u8 = 7;
const BREAK_PARTICLE_SUBDIVISIONS: i32 = 4;
const BREAK_PARTICLE_TICK_GRAVITY: f32 = 0.04;
const BREAK_PARTICLE_TICK_DRAG: f32 = 0.98;
const BREAK_PARTICLE_SCALE_MULTIPLIER: f32 = 0.2;
const BLOCK_HIT_PARTICLE_COUNT: usize = 8;
const BLOCK_BREAK_HIT_SOUND_SWING_INTERVAL: u8 = 4;
const BLOCK_BREAK_COOLDOWN_SWINGS: u8 = 5;
const ITEM_IN_HAND_SWING_DURATION_TICKS: i32 = 6;
const ITEM_IN_HAND_MAX_HEIGHT_DELTA: f32 = 0.4;
const ITEM_IN_HAND_SWING_POW_FACTOR: f32 = 4.0;
const ITEM_ICON_MESH_PIXELS: usize = 16;
const ITEM_ICON_MESH_DEPTH: f32 = 1.0 / 16.0;
const INVENTORY_PLAYER_PREVIEW_SCREEN_X: f32 = 51.0;
const INVENTORY_PLAYER_PREVIEW_SCREEN_Y: f32 = 75.0;
const INVENTORY_PLAYER_PREVIEW_CURSOR_Y_OFFSET: f32 = 50.0;
const INVENTORY_PLAYER_PREVIEW_MOUSE_DIVISOR: f32 = 40.0;
const INVENTORY_PLAYER_PREVIEW_ROTATE_SCALE_DEGREES: f32 = 20.0;
const INVENTORY_PLAYER_PREVIEW_HEAD_YAW_SCALE_DEGREES: f32 = 40.0;
const INVENTORY_PLAYER_PREVIEW_MODEL_SCALE: f32 = 2.0;
const GLASS_BLOCK_ID: u16 = 20;
const FENCE_BLOCK_ID: u16 = 85;
const NETHER_FENCE_BLOCK_ID: u16 = 113;
const COBBLE_WALL_BLOCK_ID: u16 = 139;
const BOW_ITEM_ID: u16 = 261;
const MAP_ITEM_ID: u16 = 358;
const EAT_DRINK_USE_DURATION_TICKS: f32 = 32.0;
const BOW_DRAW_DURATION_TICKS: f32 = 20.0;
const MAX_CONTINUOUS_USE_DURATION_TICKS: f32 = 20.0 * 60.0 * 60.0;
const SWORD_ITEM_IDS: [u16; 5] = [268, 272, 267, 283, 276];
const SHOVEL_ITEM_IDS: [u16; 5] = [269, 273, 256, 277, 284];
const PICKAXE_ITEM_IDS: [u16; 5] = [270, 274, 257, 285, 278];
const AXE_ITEM_IDS: [u16; 5] = [271, 275, 258, 279, 286];
const HOE_ITEM_IDS: [u16; 5] = [290, 291, 292, 293, 294];
const HAND_EQUIPPED_DIRECT_ITEM_IDS: [u16; 5] = [280, 352, 369, 346, 398];
const MIRRORED_ART_ITEM_IDS: [u16; 2] = [346, 398];
const EAT_ITEM_IDS: [u16; 22] = [
    260, 282, 297, 319, 320, 322, 349, 350, 357, 360, 363, 364, 365, 366, 367, 375, 391, 392, 393,
    394, 396, 400,
];
const DRINK_ITEM_IDS: [u16; 2] = [335, 373];
const HIT_OUTLINE_GROW: f32 = 0.002;

fn hotbar_top_offset() -> f32 {
    HOTBAR_BOTTOM_OFFSET + 22.0 * HOTBAR_GUI_SCALE
}

fn xp_bar_bottom_offset() -> f32 {
    hotbar_top_offset() + HUD_XP_BAR_TO_HOTBAR_GAP * HOTBAR_GUI_SCALE
}

fn status_row_bottom_offset() -> f32 {
    hotbar_top_offset()
        + (HUD_XP_BAR_TO_HOTBAR_GAP + HUD_XP_BAR_HEIGHT + HUD_STATUS_TO_XP_GAP) * HOTBAR_GUI_SCALE
}

#[derive(Resource)]
struct GameState {
    session: OfflineGameSession,
    blocks: BlockWorld,
}

#[derive(Resource)]
struct SaveRoot(PathBuf);

#[derive(Resource, Clone, Default)]
struct RuntimeAssets(RuntimeAssetManifest);

#[derive(Resource)]
struct BlockRenderAssets {
    opaque_material: Handle<StandardMaterial>,
    fluid_material: Handle<StandardMaterial>,
}

#[derive(Resource)]
struct UiItemRenderAssets {
    material: Handle<StandardMaterial>,
    icon_material: Handle<StandardMaterial>,
    items_material: Option<Handle<StandardMaterial>>,
    player_skin_material: Option<Handle<StandardMaterial>>,
    hand_material: Handle<StandardMaterial>,
}

#[derive(Resource, Debug, Clone, Copy)]
struct ItemInHandAnimationState {
    selected_block_id: Option<u16>,
    last_hotbar_slot: usize,
    height: f32,
    o_height: f32,
    attack_anim: f32,
    o_attack_anim: f32,
    swinging: bool,
    swing_time: i32,
    defending: bool,
    use_animation: HeldItemUseAnimation,
    use_ticks: f32,
    o_use_ticks: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum HeldItemUseAnimation {
    #[default]
    None,
    Block,
    EatDrink,
    Bow,
}

impl Default for ItemInHandAnimationState {
    fn default() -> Self {
        Self {
            selected_block_id: None,
            last_hotbar_slot: 0,
            height: 0.0,
            o_height: 0.0,
            attack_anim: 0.0,
            o_attack_anim: 0.0,
            swinging: false,
            swing_time: 0,
            defending: false,
            use_animation: HeldItemUseAnimation::None,
            use_ticks: 0.0,
            o_use_ticks: 0.0,
        }
    }
}

impl ItemInHandAnimationState {
    fn tick(&mut self, selected_slot: usize, selected_block_id: Option<u16>) {
        self.o_height = self.height;

        let mut matches =
            self.last_hotbar_slot == selected_slot && self.selected_block_id == selected_block_id;

        if self.selected_block_id.is_none() && selected_block_id.is_none() {
            matches = true;
        }

        if self.selected_block_id.is_some()
            && selected_block_id.is_some()
            && self.selected_block_id == selected_block_id
        {
            self.selected_block_id = selected_block_id;
            matches = true;
        }

        let target_height = if matches { 1.0 } else { 0.0 };
        let mut delta = target_height - self.height;
        delta = delta.clamp(
            -ITEM_IN_HAND_MAX_HEIGHT_DELTA,
            ITEM_IN_HAND_MAX_HEIGHT_DELTA,
        );
        self.height += delta;

        if self.height < 0.1 {
            self.selected_block_id = selected_block_id;
            self.last_hotbar_slot = selected_slot;
        }

        self.o_attack_anim = self.attack_anim;
        if self.swinging {
            self.swing_time += 1;
            if self.swing_time >= ITEM_IN_HAND_SWING_DURATION_TICKS {
                self.swing_time = 0;
                self.swinging = false;
            }
        } else {
            self.swing_time = 0;
        }

        self.attack_anim = self.swing_time as f32 / ITEM_IN_HAND_SWING_DURATION_TICKS as f32;
    }

    fn swing(&mut self) {
        if !self.swinging
            || self.swing_time >= ITEM_IN_HAND_SWING_DURATION_TICKS / 2
            || self.swing_time < 0
        {
            self.swing_time = -1;
            self.swinging = true;
        }
    }

    fn can_repeat_left_click_action(&self) -> bool {
        !self.swinging || self.swing_time >= ITEM_IN_HAND_SWING_DURATION_TICKS / 2
    }

    fn item_placed(&mut self) {
        self.height = 0.0;
    }

    fn attack_anim(&self, partial_tick: f32) -> f32 {
        let mut diff = self.attack_anim - self.o_attack_anim;
        if diff < 0.0 {
            diff += 1.0;
        }

        self.o_attack_anim + diff * partial_tick
    }

    fn equip_height(&self, partial_tick: f32) -> f32 {
        self.o_height + (self.height - self.o_height) * partial_tick
    }

    fn set_defending(&mut self, defending: bool) {
        self.defending = defending;
    }

    fn tick_use_animation(&mut self, use_animation: HeldItemUseAnimation) {
        self.o_use_ticks = self.use_ticks;

        if use_animation == HeldItemUseAnimation::None {
            self.use_animation = HeldItemUseAnimation::None;
            self.use_ticks = 0.0;
            self.o_use_ticks = 0.0;
            return;
        }

        if self.use_animation != use_animation {
            self.use_animation = use_animation;
            self.use_ticks = 0.0;
            self.o_use_ticks = 0.0;
        }

        let max_ticks = use_animation_max_duration_ticks(use_animation);
        self.use_ticks = (self.use_ticks + 1.0).min(max_ticks);
    }

    fn use_animation(&self) -> HeldItemUseAnimation {
        self.use_animation
    }

    fn use_ticks(&self, partial_tick: f32) -> f32 {
        self.o_use_ticks + (self.use_ticks - self.o_use_ticks) * partial_tick
    }
}

#[derive(Resource, Default)]
struct UiItemMeshCache {
    by_block_id: HashMap<u16, Handle<Mesh>>,
    by_item_icon_key: HashMap<(u16, u16), Handle<Mesh>>,
}

#[derive(Resource, Default)]
struct RuntimeAudio {
    click_sfx: Option<Handle<AudioSource>>,
    back_sfx: Option<Handle<AudioSource>>,
    pop_sfx: Option<Handle<AudioSource>>,
    wood_click_sfx: Option<Handle<AudioSource>>,
    legacy_event_sfx: HashMap<String, Vec<Handle<AudioSource>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyTileSoundProfile {
    Normal,
    Wood,
    Gravel,
    Grass,
    Metal,
    Glass,
    Cloth,
    Sand,
    Snow,
    Ladder,
    Anvil,
}

impl LegacyTileSoundProfile {
    fn break_event_key(self) -> &'static str {
        match self {
            Self::Normal | Self::Metal | Self::Anvil => "dig_stone",
            Self::Wood | Self::Ladder => "dig_wood",
            Self::Gravel => "dig_gravel",
            Self::Grass => "dig_grass",
            Self::Glass => "random_glass",
            Self::Cloth => "dig_cloth",
            Self::Sand => "dig_sand",
            Self::Snow => "dig_snow",
        }
    }

    fn place_event_key(self) -> &'static str {
        match self {
            Self::Glass => "step_stone",
            Self::Anvil => "random_anvil_land",
            _ => self.break_event_key(),
        }
    }

    fn volume(self) -> f32 {
        match self {
            Self::Anvil => 0.3,
            _ => 1.0,
        }
    }

    fn pitch(self) -> f32 {
        match self {
            Self::Metal => 1.5,
            _ => 1.0,
        }
    }
}

fn legacy_tile_sound_profile_for_block_id(block_id: u16) -> Option<LegacyTileSoundProfile> {
    let profile = match block_id {
        0 | 8 | 9 | 10 | 11 => return None,
        2 | 6 | 18 | 19 | 31 | 32 | 37 | 38 | 39 | 40 | 46 | 59 | 83 | 106 | 110 | 111 | 141
        | 142 | 170 => LegacyTileSoundProfile::Grass,
        3 | 13 | 60 | 82 => LegacyTileSoundProfile::Gravel,
        5 | 17 | 47 | 50 | 53 | 54 | 58 | 63 | 64 | 68 | 69 | 72 | 75 | 76 | 85 | 86 | 91 | 93
        | 94 | 96 | 99 | 100 | 103 | 104 | 105 | 107 | 125 | 126 | 127 | 134 | 135 | 136 | 143
        | 146 | 147 | 148 | 149 | 150 | 151 | 154 => LegacyTileSoundProfile::Wood,
        12 | 88 => LegacyTileSoundProfile::Sand,
        20 | 79 | 89 | 90 | 95 | 102 | 120 | 123 | 124 | 160 => LegacyTileSoundProfile::Glass,
        27 | 28 | 41 | 42 | 52 | 57 | 66 | 71 | 101 | 133 | 152 | 157 => {
            LegacyTileSoundProfile::Metal
        }
        35 | 80 | 81 | 92 | 171 => LegacyTileSoundProfile::Cloth,
        65 => LegacyTileSoundProfile::Ladder,
        78 => LegacyTileSoundProfile::Snow,
        145 => LegacyTileSoundProfile::Anvil,
        _ => LegacyTileSoundProfile::Normal,
    };

    Some(profile)
}

#[derive(Resource, Default)]
struct UiIconAtlasHandles {
    terrain: Option<Handle<Image>>,
    items: Option<Handle<Image>>,
}

#[derive(Resource, Default)]
struct SpawnedChunkMeshes(HashMap<ChunkPos, Entity>);

#[derive(Resource, Default)]
struct LoadedChunks(BTreeSet<ChunkPos>);

#[derive(Resource, Default)]
struct PendingChunkMeshRebuilds(BTreeSet<ChunkPos>);

#[derive(Resource, Default)]
struct CursorCaptureState {
    captured: bool,
    just_captured: bool,
}

#[derive(Resource, Default)]
struct InventoryUiState {
    open: bool,
}

#[derive(Resource, Default)]
struct InventoryDragState {
    held_stack: Option<ItemStack>,
    source_slot: Option<usize>,
}

impl InventoryDragState {
    fn clear(&mut self) {
        self.held_stack = None;
        self.source_slot = None;
    }
}

#[derive(Resource, Clone)]
struct CreativeInventoryState {
    tab: CreativeInventoryTab,
    tab_pages: [usize; CREATIVE_TABS.len()],
    tab_dynamic_groups: [usize; CREATIVE_TABS.len()],
    show_player_inventory_tab: bool,
}

impl CreativeInventoryState {
    fn active_tab_index(&self) -> usize {
        self.tab.index()
    }

    fn active_page(&self) -> usize {
        self.tab_pages[self.active_tab_index()]
    }

    fn set_active_page(&mut self, page: usize) {
        let tab_index = self.active_tab_index();
        self.tab_pages[tab_index] = page;
    }

    fn active_dynamic_group(&self) -> usize {
        self.tab_dynamic_groups[self.active_tab_index()]
    }

    fn set_active_dynamic_group(&mut self, dynamic_group: usize) {
        let tab_index = self.active_tab_index();
        self.tab_dynamic_groups[tab_index] = dynamic_group;
    }
}

impl Default for CreativeInventoryState {
    fn default() -> Self {
        Self {
            tab: CreativeInventoryTab::BuildingBlocks,
            tab_pages: [0; CREATIVE_TABS.len()],
            tab_dynamic_groups: [0; CREATIVE_TABS.len()],
            show_player_inventory_tab: false,
        }
    }
}

#[derive(Resource, Default)]
struct PauseMenuState {
    open: bool,
}

#[derive(Resource, Default)]
struct ChatInputState {
    open: bool,
    text: String,
}

#[derive(Resource, Default)]
struct SprintInputState {
    trigger_time: u8,
    trigger_registered_return: bool,
    was_running: bool,
}

#[derive(Resource, Default)]
struct BlockDestroyState {
    target: Option<BlockPos>,
    progress: f32,
    cooldown_swings: u8,
    destroy_swings: u8,
}

impl BlockDestroyState {
    fn clear(&mut self) {
        self.target = None;
        self.progress = 0.0;
        self.cooldown_swings = 0;
        self.destroy_swings = 0;
    }
}

#[derive(Resource, Default)]
struct BlockBreakOverlayState {
    entity: Option<Entity>,
    target: Option<BlockPos>,
    stage: u8,
}

#[derive(Resource, Clone, Copy)]
struct PlayerRenderPosition {
    previous: Vec3,
    current: Vec3,
}

#[derive(Resource, Default)]
struct TerrainTextureSamplerState {
    terrain_texture: Option<Handle<Image>>,
    items_texture: Option<Handle<Image>>,
    player_skin_texture: Option<Handle<Image>>,
    clouds_texture: Option<Handle<Image>>,
    configured: bool,
}

#[derive(Resource, Debug, Clone)]
struct PerfDebugConfig {
    enabled: bool,
    water_debug_enabled: bool,
    mesh_rebuild_budget_per_frame: usize,
    mesh_rebuild_warn_ms: f64,
    log_every_frames: u64,
    warn_threshold_ms: f64,
}

impl PerfDebugConfig {
    fn from_env() -> Self {
        let enabled = performance_logging_enabled();
        let water_debug_enabled =
            parse_boolean_flag(std::env::var("LCE_WATER_DEBUG").ok().as_deref());
        let mesh_rebuild_budget_per_frame =
            usize::try_from(env_u64("LCE_MESH_REBUILD_BUDGET", 1)).unwrap_or(1);
        let mesh_rebuild_warn_ms = env_f64("LCE_MESH_REBUILD_WARN_MS", 6.0).max(0.1);
        let log_every_frames = env_u64("LCE_PERF_LOG_EVERY", 30).max(1);
        let warn_threshold_ms = env_f64("LCE_PERF_WARN_MS", 8.0).max(0.1);

        Self {
            enabled,
            water_debug_enabled,
            mesh_rebuild_budget_per_frame,
            mesh_rebuild_warn_ms,
            log_every_frames,
            warn_threshold_ms,
        }
    }
}

#[derive(Resource, Debug, Default)]
struct PerfDebugState {
    update_frames: u64,
    fixed_ticks: u64,
    lifecycle_frames: u64,
}

struct WorldGenerationWorker {
    worker: WorldWorker,
}

#[derive(Resource)]
struct RuntimeLifecycle {
    controller: ChunkLifecycleController,
}

#[derive(Resource)]
struct RuntimeLifecycleHooks {
    environment: RuntimeEnvironment,
    pending_relight_chunks: BTreeSet<ChunkPos>,
    triggered_block_ticks: Vec<ScheduledTick>,
    triggered_tile_ticks: Vec<ScheduledTick>,
}

impl RuntimeLifecycleHooks {
    fn from_total_ticks(total_ticks: u64) -> Self {
        let day_time = total_ticks % DAY_LENGTH_TICKS;
        let weather = WeatherKind::Clear;

        Self {
            environment: RuntimeEnvironment {
                weather,
                day_time,
                sky_brightness: sky_brightness_for(day_time, weather),
            },
            pending_relight_chunks: BTreeSet::new(),
            triggered_block_ticks: Vec::new(),
            triggered_tile_ticks: Vec::new(),
        }
    }
}

#[derive(Resource)]
struct LookState {
    yaw_radians: f32,
    pitch_radians: f32,
}

impl Default for LookState {
    fn default() -> Self {
        Self {
            yaw_radians: 0.0,
            pitch_radians: -0.15,
        }
    }
}

#[derive(Resource, Default)]
struct LookBobState {
    pitch_old_degrees: f32,
    pitch_degrees: f32,
    yaw_old_degrees: f32,
    yaw_degrees: f32,
    x_bob_old_degrees: f32,
    x_bob_degrees: f32,
    y_bob_old_degrees: f32,
    y_bob_degrees: f32,
}

#[derive(Resource, Default)]
struct PlayerWalkAnimationState {
    walk_dist: f32,
    walk_dist_old: f32,
    bob: f32,
    bob_old: f32,
    age_ticks: f32,
}

#[derive(Debug, Default)]
struct ChunkWindowPerfStats {
    async_mode: bool,
    total: Duration,
    io: Duration,
    load_or_generate: Duration,
    mesh: Duration,
    unload: Duration,
    lifecycle: Duration,
    worker_poll: Duration,
    loaded_from_storage: usize,
    generated_sync: usize,
    requested_async: usize,
    applied_async: usize,
    unloaded_chunks: usize,
    meshes_rebuilt: usize,
    chunk_requests: usize,
    pending_async: usize,
    pending_mesh_rebuilds: usize,
    desired_loads: usize,
    deferred_loads: usize,
}

#[derive(Debug, Default)]
struct LifecyclePerfStats {
    total: Duration,
    process_ticks: Duration,
    mesh_rebuild: Duration,
    triggered_block_ticks: usize,
    triggered_tile_ticks: usize,
    fluid_tick_outcomes: usize,
    fluid_changed_chunks: usize,
    fluid_changed_blocks: usize,
    fluid_rescheduled_ticks: usize,
    redstone_tick_outcomes: usize,
    redstone_changed_chunks: usize,
    redstone_rescheduled_ticks: usize,
    relight_chunks_requested: usize,
    relight_chunks_rebuilt: usize,
}

#[derive(Component)]
struct PlayerCamera;

#[derive(Component)]
struct HotbarRootUi;

#[derive(Component)]
struct CrosshairRootUi;

#[derive(Component)]
struct ChatRootUi;

#[derive(Component)]
struct ChatInputTextUi;

#[derive(Component)]
struct HotbarSlotUi {
    slot: usize,
}

#[derive(Component)]
struct HotbarItemModelUi {
    slot: usize,
}

#[derive(Component)]
struct HotbarItemIconUi {
    slot: usize,
}

#[derive(Component)]
struct HotbarSelectionUi;

#[derive(Component)]
struct HealthHudRootUi;

#[derive(Component)]
struct HeartFillUi {
    index: usize,
}

#[derive(Component)]
struct HungerHudRootUi;

#[derive(Component)]
struct HungerFillUi {
    index: usize,
}

#[derive(Component)]
struct XpHudRootUi;

#[derive(Component)]
struct XpHudFillUi;

#[derive(Component)]
struct XpLevelTextUi;

#[derive(Component)]
struct InventoryScreenRootUi;

#[derive(Component)]
struct CreativeInventoryRootUi;

#[derive(Component)]
struct CreativeTabButtonUi {
    tab: CreativeInventoryTab,
}

#[derive(Component)]
struct CreativeInventoryPlayerTabButtonUi;

#[derive(Component)]
struct CreativeTabLabelUi;

#[derive(Component)]
struct CreativePageLabelUi;

#[derive(Component)]
struct CreativeSelectorSlotButtonUi {
    slot: usize,
}

#[derive(Component)]
struct CreativeSelectorItemIconUi {
    slot: usize,
}

#[derive(Component)]
struct CreativeHotbarSlotUi {
    slot: usize,
}

#[derive(Component)]
struct CreativeHotbarItemIconUi {
    slot: usize,
}

#[derive(Component)]
struct CreativeScrollbarThumbUi;

#[derive(Component)]
struct CreativeScrollbarArrowUi;

#[derive(Component)]
struct InventoryItemIconUi {
    slot: usize,
}

#[derive(Component)]
struct CraftingRecipeIconUi {
    recipe_id: &'static str,
}

#[derive(Component)]
struct PauseMenuRootUi;

#[derive(Component)]
struct PauseResumeButtonUi;

#[derive(Component)]
struct PauseSaveQuitButtonUi;

#[derive(Component)]
struct PauseHelpOptionsButtonUi;

#[derive(Component)]
struct PauseAchievementsButtonUi;

#[derive(Component)]
struct PauseLeaderboardsButtonUi;

#[derive(Component)]
struct PauseExitButtonUi;

#[derive(Component)]
struct LegacyMenuButtonUi {
    active: bool,
}

#[derive(Component)]
struct LegacyMenuButtonLabelUi;

#[derive(Component)]
struct PauseMenuLogoUi;

#[derive(Component)]
struct DeathScreenRootUi;

#[derive(Component)]
struct DeathRespawnButtonUi;

#[derive(Component)]
struct DeathQuitButtonUi;

#[derive(Component)]
struct DeathScoreTextUi;

#[derive(Component)]
struct FpsTextUi;

#[derive(Component)]
struct InventorySlotUi {
    slot: usize,
}

#[derive(Component)]
struct InventoryArmorSlotUi;

#[derive(Component)]
struct InventoryItemModelUi {
    slot: usize,
}

#[derive(Component)]
struct CraftingRecipeButtonUi {
    recipe_id: &'static str,
}

#[derive(Component)]
struct CraftingRecipeCountUi {
    recipe_id: &'static str,
}

#[derive(Component)]
struct CraftingRecipeModelUi {
    recipe_id: &'static str,
    recipe_index: usize,
}

#[derive(Component)]
struct CraftingStatusTextUi;

#[derive(Component)]
struct UiItemCamera;

#[derive(Component)]
struct FirstPersonItemCamera;

#[derive(Component)]
struct FirstPersonHeldItemUi;

#[derive(Component)]
struct FirstPersonHeldItemIconUi;

#[derive(Component)]
struct FirstPersonHandUi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InventoryPlayerPreviewPart {
    Head,
    Body,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

#[derive(Component)]
struct InventoryPlayerPreviewRootUi;

#[derive(Component)]
struct InventoryPlayerPreviewPartUi {
    part: InventoryPlayerPreviewPart,
}

#[derive(Component)]
struct InventoryPlayerPreviewHeldItemUi;

#[derive(Component)]
struct BreakParticle {
    velocity: Vec3,
    remaining_lifetime_ticks: f32,
}

#[derive(Component)]
struct BlockBreakOverlay;

#[derive(Component)]
struct CloudLayer;

fn main() {
    let (game_state, save_root) = create_initial_state();
    let initial_player_position = game_state.session.player().position;
    println!("Using save root: {}", save_root.0.display());
    let world_seed = game_state.session.world().seed;
    let perf_debug_config = PerfDebugConfig::from_env();
    let present_mode = default_present_mode();

    let world_generation_worker = WorldGenerationWorker {
        worker: WorldWorker::spawn_with_save_root(world_seed, save_root.0.clone()),
    };
    let runtime_lifecycle = RuntimeLifecycle {
        controller: ChunkLifecycleController::with_total_ticks(
            game_state.session.world().tick_count,
        ),
    };
    let runtime_lifecycle_hooks =
        RuntimeLifecycleHooks::from_total_ticks(game_state.session.world().tick_count);
    let runtime_assets = stage_default_runtime_assets(Path::new(".")).unwrap_or_default();

    if let Some(source_path) = runtime_assets.terrain_texture_source_path.as_ref() {
        println!("Using terrain texture source: {}", source_path.display());
    } else {
        println!("No terrain texture found; using fallback materials.");
    }

    if let Some(source_path) = runtime_assets.clouds_texture_source_path.as_ref() {
        println!("Using clouds texture source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.gui_texture_source_path.as_ref() {
        println!("Using GUI texture source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.inventory_texture_source_path.as_ref() {
        println!("Using inventory texture source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.items_texture_source_path.as_ref() {
        println!("Using items texture source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.icons_texture_source_path.as_ref() {
        println!("Using icons texture source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.font_texture_source_path.as_ref() {
        println!("Using font texture source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.mojangles_font_source_path.as_ref() {
        println!("Using Mojangles font source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.menu_logo_texture_source_path.as_ref() {
        println!("Using menu logo texture source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.player_skin_texture_source_path.as_ref() {
        println!("Using player skin source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.click_sound_source_path.as_ref() {
        println!("Using click sound source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.back_sound_source_path.as_ref() {
        println!("Using back sound source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.pop_sound_source_path.as_ref() {
        println!("Using pop sound source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.wood_click_sound_source_path.as_ref() {
        println!("Using wood click sound source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.minecraft_xgs_source_path.as_ref() {
        println!("Using minecraft xgs source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.minecraft_xsb_source_path.as_ref() {
        println!("Using minecraft xsb source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.resident_xwb_source_path.as_ref() {
        println!("Using resident xwb source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.streamed_xwb_source_path.as_ref() {
        println!("Using streamed xwb source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.additional_xsb_source_path.as_ref() {
        println!("Using additional xsb source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.additional_xwb_source_path.as_ref() {
        println!("Using additional xwb source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.additional_music_xwb_source_path.as_ref() {
        println!(
            "Using additional music xwb source: {}",
            source_path.display()
        );
    }

    if let Some(source_path) = runtime_assets.menu_sounds_xgs_source_path.as_ref() {
        println!("Using menu sounds xgs source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.menu_sounds_xsb_source_path.as_ref() {
        println!("Using menu sounds xsb source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.menu_sounds_xwb_source_path.as_ref() {
        println!("Using menu sounds xwb source: {}", source_path.display());
    }

    if let Some(source_path) = runtime_assets.legacy_event_audio_source_dir.as_ref() {
        println!(
            "Using decoded legacy event audio source: {}",
            source_path.display()
        );
    }

    if runtime_assets.minecraft_xsb_source_path.is_some()
        || runtime_assets.resident_xwb_source_path.is_some()
        || runtime_assets.streamed_xwb_source_path.is_some()
        || runtime_assets.menu_sounds_xsb_source_path.is_some()
    {
        println!(
            "XACT banks staged; most entries are XMA-coded and require decoder integration for full 1:1 playback parity."
        );
    }

    println!("Chunk generation mode: async");
    println!("Present mode: {:?}", present_mode);

    if perf_debug_config.enabled {
        println!(
            "Performance logging: enabled (every {} frames, warn threshold {:.2}ms)",
            perf_debug_config.log_every_frames, perf_debug_config.warn_threshold_ms
        );
    } else {
        println!("Performance logging: disabled (set LCE_PERF_LOG=1 to enable)");
    }

    println!(
        "Chunk mesh rebuild budget per frame: {} (mesh warn {:.2}ms)",
        perf_debug_config.mesh_rebuild_budget_per_frame, perf_debug_config.mesh_rebuild_warn_ms
    );

    if perf_debug_config.water_debug_enabled {
        println!(
            "Water debug logging: enabled (set LCE_WATER_DEBUG=1, cadence {} frames)",
            perf_debug_config.log_every_frames
        );
    }

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.45, 0.66, 0.92)))
        .insert_resource(Time::<Fixed>::from_hz(20.0))
        .insert_resource(game_state)
        .insert_resource(save_root)
        .insert_resource(CursorCaptureState::default())
        .insert_resource(InventoryUiState::default())
        .insert_resource(InventoryDragState::default())
        .insert_resource(CreativeInventoryState::default())
        .insert_resource(PauseMenuState::default())
        .insert_resource(ChatInputState::default())
        .insert_resource(SprintInputState::default())
        .insert_resource(BlockDestroyState::default())
        .insert_resource(BlockBreakOverlayState::default())
        .insert_resource(PlayerRenderPosition {
            previous: world_vec3_to_bevy(initial_player_position),
            current: world_vec3_to_bevy(initial_player_position),
        })
        .insert_resource(TerrainTextureSamplerState::default())
        .insert_resource(perf_debug_config)
        .insert_resource(PerfDebugState::default())
        .insert_non_send_resource(world_generation_worker)
        .insert_resource(runtime_lifecycle)
        .insert_resource(runtime_lifecycle_hooks)
        .insert_resource(RuntimeAssets(runtime_assets))
        .insert_resource(SpawnedChunkMeshes::default())
        .insert_resource(LoadedChunks::default())
        .insert_resource(PendingChunkMeshRebuilds::default())
        .insert_resource(UiItemMeshCache::default())
        .insert_resource(ItemInHandAnimationState::default())
        .insert_resource(LookState::default())
        .insert_resource(LookBobState::default())
        .insert_resource(PlayerWalkAnimationState::default())
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "LCE-Rust Vertical Slice".to_string(),
                        resolution: (1280.0, 720.0).into(),
                        present_mode,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_systems(Startup, setup_scene)
        .add_systems(FixedUpdate, fixed_tick_simulation)
        .add_systems(
            Update,
            (
                handle_cursor_capture,
                handle_inventory_toggle,
                handle_pause_menu_toggle,
                handle_chat_input,
                update_look_from_mouse,
                update_camera,
                render_target_block_outline,
                sync_cloud_layer,
                sync_chunk_window,
                handle_hotbar_selection,
                handle_item_drop,
                handle_debug_combat,
                handle_debug_weather,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                handle_block_interaction,
                sync_block_break_overlay,
                handle_inventory_slot_drag,
                handle_inventory_crafting,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                handle_creative_inventory_input,
                handle_creative_inventory_clicks,
            )
                .chain(),
        )
        .add_systems(Update, handle_pause_and_death_buttons)
        .add_systems(
            Update,
            sync_legacy_menu_button_visuals.after(handle_pause_and_death_buttons),
        )
        .add_systems(
            Update,
            sync_pause_and_death_ui.after(handle_pause_and_death_buttons),
        )
        .add_systems(
            Update,
            (
                tick_break_particles,
                sync_gameplay_overlay_visibility,
                sync_hotbar_ui,
                sync_first_person_item_in_hand,
                sync_health_ui,
                sync_inventory_ui,
                sync_inventory_player_preview,
                sync_creative_inventory_ui,
                sync_fps_ui,
                sync_chat_ui,
                apply_terrain_texture_sampler,
                apply_lifecycle_runtime_hooks,
                persist_on_exit,
            )
                .chain(),
        )
        .run();
}

fn create_initial_state() -> (GameState, SaveRoot) {
    let save_root = std::env::var("LCE_SAVE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SAVE_ROOT_PATH));

    let mut world_session = match load_world_snapshot(&save_root) {
        Ok(snapshot) => WorldSession {
            name: snapshot.name,
            seed: snapshot.seed,
            tick_count: snapshot.tick_count,
        },
        Err(_) => WorldSession {
            name: "DevWorld".to_string(),
            seed: 12345,
            tick_count: 0,
        },
    };

    world_session.tick_count =
        align_total_ticks_to_day_time(world_session.tick_count, DEFAULT_BOOT_DAY_TIME);

    let blocks = BlockWorld::new();
    let mut session = OfflineGameSession::new(world_session);
    session.set_player_allow_flight(false);
    seed_player_inventory(&mut session.player_mut().inventory);

    (GameState { session, blocks }, SaveRoot(save_root))
}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_sampler_state: ResMut<TerrainTextureSamplerState>,
    game: Res<GameState>,
    look: Res<LookState>,
    runtime_assets: Res<RuntimeAssets>,
) {
    setup_ui_overlay(&mut commands, &asset_server, &runtime_assets);

    let items_texture = runtime_assets
        .0
        .items_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let player_skin_texture = runtime_assets
        .0
        .player_skin_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));

    let player = game.session.player();
    let eye = Vec3::new(
        player.position.x,
        player.position.y + CAMERA_EYE_HEIGHT,
        player.position.z,
    );
    let forward = bevy_forward_from_look(&look);
    let focus = eye + forward * 8.0;

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: DEFAULT_FOV_DEGREES.to_radians(),
            ..default()
        }),
        Transform::from_translation(eye).looking_at(focus, Vec3::Y),
        PlayerCamera,
    ));

    let clouds_texture = runtime_assets
        .0
        .clouds_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));

    let (opaque_block_material, fluid_block_material, terrain_texture) =
        build_block_materials(&asset_server, &runtime_assets);
    let block_assets = BlockRenderAssets {
        opaque_material: materials.add(opaque_block_material),
        fluid_material: materials.add(fluid_block_material),
    };

    let items_material = items_texture.as_ref().map(|texture| {
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(texture.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })
    });
    let player_skin_material = player_skin_texture.as_ref().map(|texture| {
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(texture.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })
    });

    let ui_item_assets = UiItemRenderAssets {
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: terrain_texture.clone(),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        icon_material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: terrain_texture.clone(),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        }),
        items_material,
        player_skin_material,
        hand_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.93, 0.80, 0.68),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    };

    terrain_sampler_state.terrain_texture = terrain_texture.clone();
    terrain_sampler_state.items_texture = items_texture.clone();
    terrain_sampler_state.player_skin_texture = player_skin_texture.clone();
    terrain_sampler_state.clouds_texture = clouds_texture.clone();
    terrain_sampler_state.configured = false;

    spawn_cloud_layer(&mut commands, &mut meshes, &mut materials, clouds_texture);

    spawn_ui_item_models(&mut commands, &mut meshes, &ui_item_assets);

    let legacy_event_sfx = load_legacy_event_audio_handles(&asset_server, &runtime_assets.0);
    if !legacy_event_sfx.is_empty() {
        let variant_count = legacy_event_sfx.values().map(Vec::len).sum::<usize>();
        println!(
            "Loaded {variant_count} decoded legacy event wav variants across {} event keys.",
            legacy_event_sfx.len()
        );
    }

    commands.insert_resource(block_assets);
    commands.insert_resource(ui_item_assets);
    commands.insert_resource(UiIconAtlasHandles {
        terrain: terrain_texture.clone(),
        items: items_texture,
    });
    commands.insert_resource(RuntimeAudio {
        click_sfx: runtime_assets
            .0
            .click_sound_asset_path
            .as_ref()
            .map(|path| asset_server.load(path.clone())),
        back_sfx: runtime_assets
            .0
            .back_sound_asset_path
            .as_ref()
            .map(|path| asset_server.load(path.clone())),
        pop_sfx: runtime_assets
            .0
            .pop_sound_asset_path
            .as_ref()
            .map(|path| asset_server.load(path.clone())),
        wood_click_sfx: runtime_assets
            .0
            .wood_click_sound_asset_path
            .as_ref()
            .map(|path| asset_server.load(path.clone())),
        legacy_event_sfx,
    });
}

fn spawn_cloud_layer(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    clouds_texture: Option<Handle<Image>>,
) {
    let Some(clouds_texture) = clouds_texture else {
        return;
    };

    let cloud_mesh = meshes.add(build_cloud_mesh(0.0, 0.0));
    let cloud_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, CLOUD_ALPHA),
        base_color_texture: Some(clouds_texture),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        CloudLayer,
        Mesh3d(cloud_mesh),
        MeshMaterial3d(cloud_material),
        Transform::default(),
        Visibility::Visible,
    ));
}

fn spawn_ui_item_models(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    ui_item_assets: &UiItemRenderAssets,
) {
    let fallback_mesh = meshes.add(build_ui_item_mesh(1));
    let hand_mesh = meshes.add(build_first_person_hand_mesh());
    let hand_material = ui_item_assets
        .player_skin_material
        .clone()
        .unwrap_or_else(|| ui_item_assets.hand_material.clone());

    for slot in 0..HOTBAR_SLOTS {
        commands.spawn((
            HotbarItemModelUi { slot },
            Mesh3d(fallback_mesh.clone()),
            MeshMaterial3d(ui_item_assets.material.clone()),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(UI_ITEM_RENDER_LAYER),
        ));
    }

    for slot in 0..INVENTORY_SLOTS {
        commands.spawn((
            InventoryItemModelUi { slot },
            Mesh3d(fallback_mesh.clone()),
            MeshMaterial3d(ui_item_assets.material.clone()),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(UI_ITEM_RENDER_LAYER),
        ));
    }

    for (recipe_index, recipe) in RECIPES.iter().enumerate() {
        commands.spawn((
            CraftingRecipeModelUi {
                recipe_id: recipe.id,
                recipe_index,
            },
            Mesh3d(fallback_mesh.clone()),
            MeshMaterial3d(ui_item_assets.material.clone()),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(UI_ITEM_RENDER_LAYER),
        ));
    }

    commands.spawn((
        FirstPersonHeldItemUi,
        Mesh3d(fallback_mesh.clone()),
        MeshMaterial3d(ui_item_assets.material.clone()),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(FIRST_PERSON_ITEM_RENDER_LAYER),
    ));

    commands.spawn((
        FirstPersonHandUi,
        Mesh3d(hand_mesh),
        MeshMaterial3d(hand_material),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(FIRST_PERSON_ITEM_RENDER_LAYER),
    ));

    if let Some(player_skin_material) = ui_item_assets.player_skin_material.as_ref() {
        let preview_root = commands
            .spawn((
                InventoryPlayerPreviewRootUi,
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();

        commands.entity(preview_root).with_children(|preview| {
            for spec in INVENTORY_PLAYER_PREVIEW_PART_SPECS {
                let mesh = meshes.add(build_inventory_player_preview_part_mesh(spec));
                preview.spawn((
                    InventoryPlayerPreviewPartUi { part: spec.part },
                    Mesh3d(mesh),
                    MeshMaterial3d(player_skin_material.clone()),
                    Transform::from_translation(spec.position),
                    Visibility::Inherited,
                    RenderLayers::layer(UI_ITEM_RENDER_LAYER),
                ));
            }

            preview.spawn((
                InventoryPlayerPreviewHeldItemUi,
                Mesh3d(fallback_mesh.clone()),
                MeshMaterial3d(ui_item_assets.material.clone()),
                Transform::default(),
                Visibility::Hidden,
                RenderLayers::layer(UI_ITEM_RENDER_LAYER),
            ));
        });
    }
}

fn fixed_tick_simulation(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    look: Res<LookState>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    chat_input: Res<ChatInputState>,
    mut look_bob_state: ResMut<LookBobState>,
    mut player_walk_animation: ResMut<PlayerWalkAnimationState>,
    mut sprint_state: ResMut<SprintInputState>,
    mut item_in_hand_state: ResMut<ItemInHandAnimationState>,
    mut player_render_position: ResMut<PlayerRenderPosition>,
    mut game: ResMut<GameState>,
    mut lifecycle: ResMut<RuntimeLifecycle>,
    mut lifecycle_hooks: ResMut<RuntimeLifecycleHooks>,
    perf_config: Res<PerfDebugConfig>,
    mut perf_state: ResMut<PerfDebugState>,
) {
    let tick_start = Instant::now();

    if pause_menu.open {
        return;
    }

    let pitch_degrees = look.pitch_radians.to_degrees();
    let yaw_degrees = look.yaw_radians.to_degrees();
    look_bob_state.pitch_old_degrees = look_bob_state.pitch_degrees;
    look_bob_state.pitch_degrees = pitch_degrees;
    look_bob_state.yaw_old_degrees = look_bob_state.yaw_degrees;
    look_bob_state.yaw_degrees = yaw_degrees;

    look_bob_state.x_bob_old_degrees = look_bob_state.x_bob_degrees;
    look_bob_state.y_bob_old_degrees = look_bob_state.y_bob_degrees;
    look_bob_state.x_bob_degrees += (pitch_degrees - look_bob_state.x_bob_degrees) * 0.5;
    look_bob_state.y_bob_degrees += (yaw_degrees - look_bob_state.y_bob_degrees) * 0.5;

    let mut input = MovementInput::default();
    let mut local_strafe = 0.0;
    let mut local_forward = 0.0;

    if !inventory_ui.open && !chat_input.open {
        if keys.pressed(KeyCode::KeyA) {
            local_strafe -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            local_strafe += 1.0;
        }
        if keys.pressed(KeyCode::KeyW) {
            local_forward += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            local_forward -= 1.0;
        }
    }

    let (world_strafe, world_forward) =
        movement_axes_from_yaw(look.yaw_radians, local_strafe, local_forward);
    input.strafe = world_strafe;
    input.forward = world_forward;

    let sneaking_held = !inventory_ui.open
        && !chat_input.open
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight));
    input.sneak = sneaking_held;

    let sprint_modifier_held =
        keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let is_using_item = !inventory_ui.open
        && !chat_input.open
        && mouse.pressed(MouseButton::Right)
        && game
            .session
            .player()
            .inventory
            .selected_stack()
            .is_some_and(|stack| {
                held_item_use_animation_for_item(stack.item_id) != HeldItemUseAnimation::None
            });
    update_sprint_from_cxx_rules(
        &mut game.session,
        &mut sprint_state,
        local_forward,
        sprint_modifier_held,
        sneaking_held,
        is_using_item,
    );

    if !inventory_ui.open && !chat_input.open {
        if keys.just_pressed(KeyCode::Space) {
            game.session.register_jump_tap();
        }

        if keys.pressed(KeyCode::Space) {
            input.jump = true;
        }
    }

    {
        let GameState { session, blocks } = &mut *game;

        player_render_position.previous = player_render_position.current;

        session.tick_with_collision_and_water(
            input,
            |block| is_solid_block_for_player_collision(blocks.block_id(block)),
            |block| {
                let block_id = blocks.block_id(block);
                block_id == WATER_SOURCE_BLOCK_ID || block_id == WATER_FLOWING_BLOCK_ID
            },
        );

        let player_position = session.player().position;
        player_render_position.current = world_vec3_to_bevy(player_position);

        player_walk_animation.walk_dist_old = player_walk_animation.walk_dist;
        player_walk_animation.bob_old = player_walk_animation.bob;
        player_walk_animation.age_ticks += 1.0;

        let x_delta = player_render_position.current.x - player_render_position.previous.x;
        let z_delta = player_render_position.current.z - player_render_position.previous.z;
        let horizontal_movement = (x_delta * x_delta + z_delta * z_delta).sqrt();
        player_walk_animation.walk_dist += horizontal_movement * 0.6;

        let mut target_bob = horizontal_movement;
        if target_bob > 0.1 {
            target_bob = 0.1;
        }

        let player = session.player();
        if !player.on_ground || player.health <= 0 {
            target_bob = 0.0;
        }

        player_walk_animation.bob += (target_bob - player_walk_animation.bob) * 0.4;

        let selected_slot = session.player().inventory.selected_hotbar_slot();
        let selected_stack = session.player().inventory.selected_stack();
        let selected_block_id = selected_stack.and_then(|stack| block_id_for_item(stack.item_id));
        item_in_hand_state.tick(selected_slot, selected_block_id);

        let use_animation =
            if !inventory_ui.open && !chat_input.open && mouse.pressed(MouseButton::Right) {
                selected_stack
                    .map(|stack| held_item_use_animation_for_item(stack.item_id))
                    .unwrap_or(HeldItemUseAnimation::None)
            } else {
                HeldItemUseAnimation::None
            };

        item_in_hand_state.tick_use_animation(use_animation);
        item_in_hand_state.set_defending(use_animation == HeldItemUseAnimation::Block);
    }
    lifecycle.controller.tick_once();
    consume_runtime_lifecycle_events(&mut lifecycle, &mut lifecycle_hooks);

    if perf_config.enabled {
        perf_state.fixed_ticks = perf_state.fixed_ticks.saturating_add(1);
        let elapsed_ms = tick_start.elapsed().as_secs_f64() * 1000.0;
        let periodic = perf_state.fixed_ticks % perf_config.log_every_frames == 0;
        let slow = elapsed_ms >= perf_config.warn_threshold_ms;

        if periodic || slow {
            let log_line = format!(
                "perf fixed_tick #{}: total={elapsed_ms:.2}ms input(strafe={:.2},forward={:.2},jump={}) player=({:.2},{:.2},{:.2})",
                perf_state.fixed_ticks,
                input.strafe,
                input.forward,
                input.jump,
                game.session.player().position.x,
                game.session.player().position.y,
                game.session.player().position.z,
            );

            if slow {
                warn!("{log_line}");
            } else {
                info!("{log_line}");
            }
        }
    }
}

fn update_look_from_mouse(
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut look: ResMut<LookState>,
    cursor_capture: Res<CursorCaptureState>,
) {
    let mut total_delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        total_delta += event.delta;
    }

    if !cursor_capture.captured || total_delta == Vec2::ZERO {
        return;
    }

    look.yaw_radians += total_delta.x * LOOK_SENSITIVITY_RADIANS_PER_PIXEL;
    look.pitch_radians = (look.pitch_radians - total_delta.y * LOOK_SENSITIVITY_RADIANS_PER_PIXEL)
        .clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);
}

fn handle_inventory_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    game: Res<GameState>,
    mut inventory_ui: ResMut<InventoryUiState>,
    mut creative_ui: ResMut<CreativeInventoryState>,
    pause_menu: Res<PauseMenuState>,
    chat_input: Res<ChatInputState>,
    mut commands: Commands,
    runtime_audio: Res<RuntimeAudio>,
    mut capture_state: ResMut<CursorCaptureState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if chat_input.open {
        return;
    }

    if game.session.player().is_dead || pause_menu.open {
        if inventory_ui.open {
            inventory_ui.open = false;
            creative_ui.show_player_inventory_tab = false;
        }
        return;
    }

    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    if keys.just_pressed(KeyCode::KeyE) {
        inventory_ui.open = !inventory_ui.open;

        if inventory_ui.open {
            creative_ui.show_player_inventory_tab = false;
            release_cursor(&mut window);
            capture_state.captured = false;
            capture_state.just_captured = false;
            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.28);
        } else {
            creative_ui.show_player_inventory_tab = false;
            capture_state.captured = capture_cursor(&mut window);
            capture_state.just_captured = capture_state.captured;
            play_sound(
                &mut commands,
                runtime_audio
                    .back_sfx
                    .as_ref()
                    .or(runtime_audio.click_sfx.as_ref()),
                0.28,
            );
        }

        return;
    }

    if inventory_ui.open && keys.just_pressed(KeyCode::Escape) {
        inventory_ui.open = false;
        creative_ui.show_player_inventory_tab = false;
        capture_state.captured = capture_cursor(&mut window);
        capture_state.just_captured = capture_state.captured;
        play_sound(
            &mut commands,
            runtime_audio
                .back_sfx
                .as_ref()
                .or(runtime_audio.click_sfx.as_ref()),
            0.28,
        );
    }
}

fn handle_pause_menu_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    chat_input: Res<ChatInputState>,
    mut pause_menu: ResMut<PauseMenuState>,
    mut commands: Commands,
    runtime_audio: Res<RuntimeAudio>,
    mut capture_state: ResMut<CursorCaptureState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if game.session.player().is_dead || inventory_ui.open || chat_input.open {
        return;
    }

    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    if pause_menu.open {
        pause_menu.open = false;
        capture_state.captured = capture_cursor(&mut window);
        capture_state.just_captured = capture_state.captured;
        play_sound(
            &mut commands,
            runtime_audio
                .back_sfx
                .as_ref()
                .or(runtime_audio.click_sfx.as_ref()),
            0.30,
        );
    } else {
        pause_menu.open = true;
        release_cursor(&mut window);
        capture_state.captured = false;
        capture_state.just_captured = false;
        play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.30);
    }
}

fn handle_chat_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard_input_events: EventReader<KeyboardInput>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    mut game: ResMut<GameState>,
    mut chat_input: ResMut<ChatInputState>,
    mut capture_state: ResMut<CursorCaptureState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if inventory_ui.open || pause_menu.open || game.session.player().is_dead {
        chat_input.open = false;
        chat_input.text.clear();
        return;
    }

    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    if !chat_input.open {
        let opened_with_slash = keys.just_pressed(KeyCode::Slash);
        let opened_with_enter =
            keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter);
        let opened_with_chat = keys.just_pressed(KeyCode::KeyT);

        if opened_with_slash || opened_with_enter || opened_with_chat {
            chat_input.open = true;
            chat_input.text.clear();
            if opened_with_slash {
                chat_input.text.push('/');
            }
            release_cursor(&mut window);
            capture_state.captured = false;
            capture_state.just_captured = false;
            keyboard_input_events.clear();
        }
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        close_chat_input(&mut chat_input, &mut capture_state, &mut window);
        return;
    }

    let mut submit = false;
    for event in keyboard_input_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match &event.logical_key {
            Key::Enter => {
                submit = true;
                break;
            }
            Key::Backspace => {
                chat_input.text.pop();
            }
            Key::Character(value) => {
                if value.chars().any(|character| character.is_control()) {
                    continue;
                }
                chat_input.text.push_str(value);
            }
            Key::Space => {
                chat_input.text.push(' ');
            }
            _ => {}
        }
    }

    if submit {
        let command = chat_input.text.trim().to_string();
        execute_chat_command(&command, &mut game);
        close_chat_input(&mut chat_input, &mut capture_state, &mut window);
    }
}

fn close_chat_input(
    chat_input: &mut ChatInputState,
    capture_state: &mut CursorCaptureState,
    window: &mut Window,
) {
    chat_input.open = false;
    chat_input.text.clear();
    capture_state.captured = capture_cursor(window);
    capture_state.just_captured = capture_state.captured;
}

fn execute_chat_command(command_text: &str, game: &mut GameState) {
    if command_text.is_empty() || !command_text.starts_with('/') {
        return;
    }

    let mut command_parts = command_text[1..].split_whitespace();
    let Some(command_name) = command_parts.next() else {
        return;
    };

    if command_name.eq_ignore_ascii_case("gamemode") || command_name.eq_ignore_ascii_case("gm") {
        let Some(mode_raw) = command_parts.next() else {
            let current_mode = if game.session.player().allow_flight {
                "creative"
            } else {
                "survival"
            };
            println!("Current gamemode: {current_mode}");
            return;
        };

        let mode = mode_raw.to_ascii_lowercase();
        match mode.as_str() {
            "0" | "s" | "survival" => {
                game.session.set_player_allow_flight(false);
                println!("Gamemode set to survival");
            }
            "1" | "c" | "creative" => {
                game.session.set_player_allow_flight(true);
                println!("Gamemode set to creative");
            }
            _ => {
                println!("Unknown gamemode '{mode_raw}'. Use survival or creative.");
            }
        }
    } else {
        println!("Unknown command '{command_name}'");
    }
}

fn handle_cursor_capture(
    mouse: Res<ButtonInput<MouseButton>>,
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    chat_input: Res<ChatInputState>,
    mut capture_state: ResMut<CursorCaptureState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    capture_state.just_captured = false;

    if chat_input.open {
        return;
    }

    if !allow_cursor_capture(
        inventory_ui.open,
        pause_menu.open,
        game.session.player().is_dead,
    ) {
        return;
    }

    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) && !capture_state.captured {
        capture_state.captured = capture_cursor(&mut window);
        capture_state.just_captured = capture_state.captured;
    }
}

fn release_cursor(window: &mut Window) {
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}

fn capture_cursor(window: &mut Window) -> bool {
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    if window.cursor_options.grab_mode == CursorGrabMode::None {
        window.cursor_options.grab_mode = CursorGrabMode::Confined;
    }
    window.cursor_options.visible = false;
    window.cursor_options.grab_mode != CursorGrabMode::None
}

fn update_camera(
    fixed_time: Res<Time<Fixed>>,
    game: Res<GameState>,
    player_walk_animation: Res<PlayerWalkAnimationState>,
    player_render_position: Res<PlayerRenderPosition>,
    look: Res<LookState>,
    mut camera_query: Query<&mut Transform, With<PlayerCamera>>,
) {
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };

    let alpha = fixed_time.overstep_fraction().clamp(0.0, 1.0);
    let player = player_render_position
        .previous
        .lerp(player_render_position.current, alpha);
    let player_state = game.session.player();

    let mut eye = Vec3::new(player.x, player.y + CAMERA_EYE_HEIGHT, player.z);
    let forward = bevy_forward_from_look(&look);

    if player_state.on_ground && !player_state.is_flying {
        let walk_dist = player_walk_animation.walk_dist_old
            + (player_walk_animation.walk_dist - player_walk_animation.walk_dist_old) * alpha;
        let bob = player_walk_animation.bob_old
            + (player_walk_animation.bob - player_walk_animation.bob_old) * alpha;
        let phase = -walk_dist * std::f32::consts::PI;
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        eye += right * (phase.sin() * bob * 0.5);
        eye += Vec3::Y * (-(phase.cos().abs()) * bob);

        let focus = eye + forward * 8.0;
        let mut transformed = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
        let roll = (phase.sin() * bob * 3.0).to_radians();
        let pitch = (phase.cos().abs() * bob * 5.0).to_radians();
        transformed.rotation =
            transformed.rotation * Quat::from_rotation_z(roll) * Quat::from_rotation_x(pitch);
        *camera_transform = transformed;
        return;
    }

    let focus = eye + forward * 8.0;
    *camera_transform = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
}

fn sync_cloud_layer(
    fixed_time: Res<Time<Fixed>>,
    lifecycle: Res<RuntimeLifecycle>,
    game: Res<GameState>,
    player_render_position: Res<PlayerRenderPosition>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cloud_query: Query<(&mut Transform, &Mesh3d, &mut Visibility), With<CloudLayer>>,
) {
    let alpha = fixed_time.overstep_fraction().clamp(0.0, 1.0);
    let player = player_render_position
        .previous
        .lerp(player_render_position.current, alpha);

    let tick_time = cloud_tick_time(lifecycle.controller.time().total_ticks, alpha);
    let cloud_motion = cloud_uv_motion(f64::from(player.x), f64::from(player.z), tick_time);
    let y = cloud_world_y(player.y);
    let eye_block = BlockPos::new(
        player.x.floor() as i32,
        (player.y + CAMERA_EYE_HEIGHT).floor() as i32,
        player.z.floor() as i32,
    );
    let cloud_visible = clouds_visible_for_camera_block(game.blocks.block_id(eye_block));

    for (mut transform, mesh_3d, mut visibility) in &mut cloud_query {
        transform.translation = Vec3::new(
            player.x - cloud_motion.x_offset_blocks,
            y,
            player.z - cloud_motion.z_offset_blocks,
        );
        *visibility = if cloud_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        let Some(mesh) = meshes.get_mut(&mesh_3d.0) else {
            continue;
        };
        update_cloud_mesh_uvs(mesh, cloud_motion.u_offset, cloud_motion.v_offset);
    }
}

fn update_cloud_mesh_uvs(mesh: &mut Mesh, u_offset: f32, v_offset: f32) {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned()
    else {
        return;
    };

    let Some(VertexAttributeValues::Float32x3(normals)) =
        mesh.attribute(Mesh::ATTRIBUTE_NORMAL).cloned()
    else {
        return;
    };

    if positions.len() != normals.len() || positions.len() % 4 != 0 {
        return;
    }

    let mut uvs = Vec::with_capacity(positions.len());
    for quad_start in (0..positions.len()).step_by(4) {
        let mut center_x = 0.0;
        let mut center_z = 0.0;
        for vertex_index in quad_start..(quad_start + 4) {
            center_x += positions[vertex_index][0];
            center_z += positions[vertex_index][2];
        }
        center_x *= 0.25;
        center_z *= 0.25;

        let normal = normals[quad_start];
        if normal[0] > 0.5 {
            center_x -= CLOUD_ADVANCED_TEXEL_WORLD_SIZE * 0.5;
        } else if normal[0] < -0.5 {
            center_x += CLOUD_ADVANCED_TEXEL_WORLD_SIZE * 0.5;
        }

        if normal[2] > 0.5 {
            center_z -= CLOUD_ADVANCED_TEXEL_WORLD_SIZE * 0.5;
        } else if normal[2] < -0.5 {
            center_z += CLOUD_ADVANCED_TEXEL_WORLD_SIZE * 0.5;
        }

        let texel_x = (center_x / CLOUD_ADVANCED_TEXEL_WORLD_SIZE).floor();
        let texel_z = (center_z / CLOUD_ADVANCED_TEXEL_WORLD_SIZE).floor();
        let u = (texel_x + 0.5) * CLOUD_TEXEL_UV_SCALE + u_offset;
        let v = (texel_z + 0.5) * CLOUD_TEXEL_UV_SCALE + v_offset;

        uvs.push([u, v]);
        uvs.push([u, v]);
        uvs.push([u, v]);
        uvs.push([u, v]);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

fn apply_terrain_texture_sampler(
    mut sampler_state: ResMut<TerrainTextureSamplerState>,
    mut images: ResMut<Assets<Image>>,
) {
    if sampler_state.configured {
        return;
    }

    let mut saw_texture_handle = false;
    let mut any_pending = false;

    for texture_handle in [
        sampler_state.terrain_texture.as_ref(),
        sampler_state.items_texture.as_ref(),
        sampler_state.player_skin_texture.as_ref(),
        sampler_state.clouds_texture.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        saw_texture_handle = true;

        let Some(image) = images.get_mut(texture_handle) else {
            any_pending = true;
            continue;
        };

        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            min_filter: ImageFilterMode::Nearest,
            mag_filter: ImageFilterMode::Nearest,
            mipmap_filter: ImageFilterMode::Nearest,
            ..default()
        });
    }

    sampler_state.configured = !saw_texture_handle || !any_pending;
}

fn update_sprint_from_cxx_rules(
    session: &mut OfflineGameSession,
    sprint_state: &mut SprintInputState,
    local_forward: f32,
    sprint_modifier_held: bool,
    sneaking: bool,
    using_item: bool,
) {
    if sprint_state.trigger_time > 0 {
        sprint_state.trigger_time = sprint_state.trigger_time.saturating_sub(1);
    }

    if session.player().is_dead {
        session.set_player_sprinting(false);
        sprint_state.was_running = false;
        sprint_state.trigger_time = 0;
        sprint_state.trigger_registered_return = false;
        return;
    }

    let enough_food_to_sprint = true;
    let running_now = local_forward >= SPRINT_RUN_THRESHOLD;
    let on_ground = session.player().on_ground;
    let flying_now = session.player().allow_flight && session.player().is_flying;

    if on_ground
        && !session.player().is_sprinting
        && enough_food_to_sprint
        && !sneaking
        && !using_item
    {
        if !sprint_state.was_running && running_now {
            if sprint_state.trigger_time == 0 {
                sprint_state.trigger_time = SPRINT_TRIGGER_WINDOW_TICKS;
                sprint_state.trigger_registered_return = false;
            } else if sprint_state.trigger_registered_return {
                session.set_player_sprinting(true);
                sprint_state.trigger_time = 0;
                sprint_state.trigger_registered_return = false;
            }
        } else if sprint_state.trigger_time > 0 && local_forward == 0.0 {
            sprint_state.trigger_registered_return = true;
        }
    }

    let keyboard_sprint = sprint_modifier_held
        && local_forward > 0.0
        && on_ground
        && enough_food_to_sprint
        && !sneaking
        && !using_item;
    if keyboard_sprint {
        session.set_player_sprinting(true);
    }

    let keep_sprinting = running_now || (flying_now && sprint_modifier_held && local_forward > 0.0);
    if session.player().is_sprinting
        && (!keep_sprinting || !enough_food_to_sprint || sneaking || using_item)
    {
        session.set_player_sprinting(false);
    }

    sprint_state.was_running = running_now;
}

fn sync_chunk_window(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut game: ResMut<GameState>,
    perf_config: Res<PerfDebugConfig>,
    mut perf_state: ResMut<PerfDebugState>,
    mut world_generation_worker: NonSendMut<WorldGenerationWorker>,
    mut lifecycle: ResMut<RuntimeLifecycle>,
    mut lifecycle_hooks: ResMut<RuntimeLifecycleHooks>,
    block_assets: Res<BlockRenderAssets>,
    mut spawned_chunk_meshes: ResMut<SpawnedChunkMeshes>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    mut pending_mesh_rebuilds: ResMut<PendingChunkMeshRebuilds>,
) {
    let frame_start = Instant::now();
    let mut perf_stats = ChunkWindowPerfStats {
        async_mode: true,
        ..ChunkWindowPerfStats::default()
    };

    let player_chunk = player_chunk_from_position(game.session.player().position);
    let desired_chunks = desired_chunk_window(player_chunk, CHUNK_LOAD_RADIUS);
    let (mut to_load, to_unload) = chunk_diff(&loaded_chunks.0, &desired_chunks);
    sort_chunks_by_distance(&mut to_load, player_chunk);
    perf_stats.desired_loads = to_load.len();

    let mut chunk_request_budget = MAX_CHUNK_REQUESTS_PER_FRAME;

    for chunk in to_load.iter().copied() {
        if chunk_request_budget == 0 {
            break;
        }

        if world_generation_worker.worker.is_chunk_pending(chunk) {
            continue;
        }

        if world_generation_worker.worker.pending_count() >= MAX_PENDING_CHUNK_REQUESTS {
            break;
        }

        if world_generation_worker
            .worker
            .request_chunk_generation(chunk)
        {
            chunk_request_budget -= 1;
            perf_stats.chunk_requests += 1;
            perf_stats.requested_async += 1;
        }
    }

    for chunk in to_unload {
        let unload_start = Instant::now();
        if let Some(entity) = spawned_chunk_meshes.0.remove(&chunk) {
            commands.entity(entity).despawn_recursive();
        }

        pending_mesh_rebuilds.0.remove(&chunk);

        game.blocks.unload_chunk(chunk);
        world_generation_worker.worker.cancel_chunk_interest(chunk);
        lifecycle_note_chunk_unloaded(&mut lifecycle.controller, chunk);
        loaded_chunks.0.remove(&chunk);

        for neighbor in chunk_with_neighbors(chunk) {
            if !loaded_chunks.0.contains(&neighbor) {
                continue;
            }

            pending_mesh_rebuilds.0.insert(neighbor);
        }

        perf_stats.unloaded_chunks += 1;
        perf_stats.unload += unload_start.elapsed();
    }

    let poll_start = Instant::now();
    let generated_chunks = world_generation_worker
        .worker
        .poll_generated_chunks_with_limit(MAX_GENERATED_CHUNKS_APPLIED_PER_FRAME);
    perf_stats.worker_poll += poll_start.elapsed();

    for generated in generated_chunks {
        if !desired_chunks.contains(&generated.chunk) {
            continue;
        }

        if loaded_chunks.0.contains(&generated.chunk) {
            continue;
        }

        let chunk = generated.chunk;
        match generated.source {
            ChunkDataSource::Storage => perf_stats.loaded_from_storage += 1,
            ChunkDataSource::Generated => perf_stats.generated_sync += 1,
        }

        let apply_start = Instant::now();
        apply_generated_chunk(&mut game.blocks, generated);
        perf_stats.load_or_generate += apply_start.elapsed();
        perf_stats.applied_async += 1;

        lifecycle_note_chunk_loaded(&mut lifecycle.controller, chunk);
        loaded_chunks.0.insert(chunk);

        pending_mesh_rebuilds.0.remove(&chunk);

        let mesh_start = Instant::now();
        rebuild_chunk_mesh_entity(
            &mut commands,
            &mut meshes,
            &mut spawned_chunk_meshes,
            &block_assets,
            &game.blocks,
            chunk,
        );
        let elapsed = mesh_start.elapsed();
        perf_stats.mesh += elapsed;
        perf_stats.meshes_rebuilt += 1;
        maybe_log_mesh_rebuild_spike(&perf_config, "stream_load_current", chunk, elapsed);

        for neighbor in chunk_with_neighbors(chunk) {
            if neighbor == chunk {
                continue;
            }

            if !loaded_chunks.0.contains(&neighbor) {
                continue;
            }

            pending_mesh_rebuilds.0.insert(neighbor);
        }
    }

    let queued_rebuild_chunks: Vec<_> = pending_mesh_rebuilds
        .0
        .iter()
        .copied()
        .take(perf_config.mesh_rebuild_budget_per_frame)
        .collect();

    for chunk in queued_rebuild_chunks {
        pending_mesh_rebuilds.0.remove(&chunk);

        if !loaded_chunks.0.contains(&chunk) {
            continue;
        }

        let mesh_start = Instant::now();
        rebuild_chunk_mesh_entity(
            &mut commands,
            &mut meshes,
            &mut spawned_chunk_meshes,
            &block_assets,
            &game.blocks,
            chunk,
        );
        let elapsed = mesh_start.elapsed();
        perf_stats.mesh += elapsed;
        perf_stats.meshes_rebuilt += 1;
        maybe_log_mesh_rebuild_spike(&perf_config, "stream_queued", chunk, elapsed);
    }

    let lifecycle_start = Instant::now();
    consume_runtime_lifecycle_events(&mut lifecycle, &mut lifecycle_hooks);
    perf_stats.lifecycle += lifecycle_start.elapsed();

    perf_stats.pending_async = world_generation_worker.worker.pending_count();
    perf_stats.pending_mesh_rebuilds = pending_mesh_rebuilds.0.len();
    perf_stats.deferred_loads = to_load
        .len()
        .saturating_sub(perf_stats.loaded_from_storage + perf_stats.requested_async);
    perf_stats.total = frame_start.elapsed();
    maybe_log_chunk_window_perf(&perf_config, &mut perf_state, &perf_stats);
}

fn maybe_log_chunk_window_perf(
    perf_config: &PerfDebugConfig,
    perf_state: &mut PerfDebugState,
    stats: &ChunkWindowPerfStats,
) {
    if !perf_config.enabled {
        return;
    }

    perf_state.update_frames = perf_state.update_frames.saturating_add(1);
    let total_ms = stats.total.as_secs_f64() * 1000.0;
    let periodic = perf_state.update_frames % perf_config.log_every_frames == 0;
    let slow = total_ms >= perf_config.warn_threshold_ms;

    if !periodic && !slow {
        return;
    }

    let mode = if stats.async_mode { "async" } else { "sync" };
    let log_line = format!(
        "perf chunk_window #{} [{}]: total={total_ms:.2}ms io={:.2}ms load_or_generate={:.2}ms mesh={:.2}ms unload={:.2}ms worker_poll={:.2}ms lifecycle={:.2}ms counts(load={},gen_sync={},req_async={},apply_async={},unload={},mesh_rebuilds={},mesh_queue={},requests={},pending={},desired_loads={},deferred_loads={})",
        perf_state.update_frames,
        mode,
        stats.io.as_secs_f64() * 1000.0,
        stats.load_or_generate.as_secs_f64() * 1000.0,
        stats.mesh.as_secs_f64() * 1000.0,
        stats.unload.as_secs_f64() * 1000.0,
        stats.worker_poll.as_secs_f64() * 1000.0,
        stats.lifecycle.as_secs_f64() * 1000.0,
        stats.loaded_from_storage,
        stats.generated_sync,
        stats.requested_async,
        stats.applied_async,
        stats.unloaded_chunks,
        stats.meshes_rebuilt,
        stats.pending_mesh_rebuilds,
        stats.chunk_requests,
        stats.pending_async,
        stats.desired_loads,
        stats.deferred_loads,
    );

    if slow {
        warn!("{log_line}");
    } else {
        info!("{log_line}");
    }
}

fn maybe_log_lifecycle_perf(
    perf_config: &PerfDebugConfig,
    perf_state: &mut PerfDebugState,
    stats: &LifecyclePerfStats,
) {
    if !perf_config.enabled && !perf_config.water_debug_enabled {
        return;
    }

    perf_state.lifecycle_frames = perf_state.lifecycle_frames.saturating_add(1);
    let total_ms = stats.total.as_secs_f64() * 1000.0;
    let periodic = perf_state.lifecycle_frames % perf_config.log_every_frames == 0;
    let slow = total_ms >= perf_config.warn_threshold_ms;
    let active = stats.fluid_tick_outcomes > 0 || stats.relight_chunks_rebuilt > 0;

    if !slow && !periodic && !(perf_config.water_debug_enabled && active) {
        return;
    }

    let log_line = format!(
        "perf lifecycle #{}: total={total_ms:.2}ms tick_process={:.2}ms mesh_rebuild={:.2}ms ticks(block={},tile={},fluid_outcomes={},fluid_chunks={},fluid_blocks={},fluid_rescheduled={},redstone_outcomes={},redstone_chunks={},redstone_rescheduled={}) relight(requested={},rebuilt={})",
        perf_state.lifecycle_frames,
        stats.process_ticks.as_secs_f64() * 1000.0,
        stats.mesh_rebuild.as_secs_f64() * 1000.0,
        stats.triggered_block_ticks,
        stats.triggered_tile_ticks,
        stats.fluid_tick_outcomes,
        stats.fluid_changed_chunks,
        stats.fluid_changed_blocks,
        stats.fluid_rescheduled_ticks,
        stats.redstone_tick_outcomes,
        stats.redstone_changed_chunks,
        stats.redstone_rescheduled_ticks,
        stats.relight_chunks_requested,
        stats.relight_chunks_rebuilt,
    );

    if slow {
        warn!("{log_line}");
    } else {
        info!("{log_line}");
    }
}

fn maybe_log_mesh_rebuild_spike(
    perf_config: &PerfDebugConfig,
    reason: &str,
    chunk: ChunkPos,
    elapsed: Duration,
) {
    if !perf_config.water_debug_enabled {
        return;
    }

    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    if elapsed_ms < perf_config.mesh_rebuild_warn_ms {
        return;
    }

    warn!(
        "perf mesh_rebuild [{reason}]: chunk=({}, {}) took={elapsed_ms:.2}ms",
        chunk.x, chunk.z
    );
}

fn sort_chunks_by_distance(chunks: &mut [ChunkPos], center: ChunkPos) {
    chunks.sort_by_key(|chunk| chunk_distance_squared(*chunk, center));
}

fn chunk_with_neighbors(center: ChunkPos) -> [ChunkPos; 9] {
    [
        ChunkPos::new(center.x - 1, center.z - 1),
        ChunkPos::new(center.x, center.z - 1),
        ChunkPos::new(center.x + 1, center.z - 1),
        ChunkPos::new(center.x - 1, center.z),
        center,
        ChunkPos::new(center.x + 1, center.z),
        ChunkPos::new(center.x - 1, center.z + 1),
        ChunkPos::new(center.x, center.z + 1),
        ChunkPos::new(center.x + 1, center.z + 1),
    ]
}

fn chunk_distance_squared(chunk: ChunkPos, center: ChunkPos) -> i64 {
    let dx = i64::from(chunk.x) - i64::from(center.x);
    let dz = i64::from(chunk.z) - i64::from(center.z);
    (dx * dx) + (dz * dz)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .unwrap_or(default)
}

fn handle_hotbar_selection(
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    chat_input: Res<ChatInputState>,
    mut commands: Commands,
    runtime_audio: Res<RuntimeAudio>,
    mut game: ResMut<GameState>,
) {
    if chat_input.open || inventory_ui.open || pause_menu.open || game.session.player().is_dead {
        return;
    }

    let mut selected_slot = if keys.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(3)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(4)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(5)
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(6)
    } else if keys.just_pressed(KeyCode::Digit8) {
        Some(7)
    } else if keys.just_pressed(KeyCode::Digit9) {
        Some(8)
    } else {
        None
    };

    if selected_slot.is_none() {
        let mut wheel_delta = 0.0_f32;
        for event in mouse_wheel_events.read() {
            wheel_delta += event.y;
        }

        let wheel_steps = if wheel_delta > 0.0 {
            wheel_delta.ceil() as i32
        } else if wheel_delta < 0.0 {
            wheel_delta.floor() as i32
        } else {
            0
        };

        if wheel_steps != 0 {
            let current = i32::try_from(game.session.player().inventory.selected_hotbar_slot())
                .expect("hotbar slot index should fit i32");
            let hotbar_slots =
                i32::try_from(HOTBAR_SLOTS).expect("hotbar slot count should fit i32");
            let next = (current - wheel_steps).rem_euclid(hotbar_slots) as usize;
            selected_slot = Some(next);
        }
    }

    if let Some(slot) = selected_slot
        && slot < HOTBAR_SLOTS
    {
        if game.session.player().inventory.selected_hotbar_slot() != slot {
            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.28);
        }
        let _ = game.session.player_mut().inventory.select_hotbar_slot(slot);
    }
}

fn handle_item_drop(
    keys: Res<ButtonInput<KeyCode>>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    chat_input: Res<ChatInputState>,
    mut commands: Commands,
    runtime_audio: Res<RuntimeAudio>,
    mut game: ResMut<GameState>,
) {
    if chat_input.open || inventory_ui.open || pause_menu.open || game.session.player().is_dead {
        return;
    }

    if keys.just_pressed(KeyCode::KeyQ) {
        let consumed = game.session.player_mut().inventory.consume_selected(1);
        if consumed {
            play_sound(
                &mut commands,
                runtime_audio
                    .pop_sfx
                    .as_ref()
                    .or(runtime_audio.click_sfx.as_ref()),
                0.30,
            );
        }
    }
}

fn handle_debug_combat(
    keys: Res<ButtonInput<KeyCode>>,
    chat_input: Res<ChatInputState>,
    mut game: ResMut<GameState>,
) {
    if chat_input.open {
        return;
    }

    if keys.just_pressed(KeyCode::KeyK) {
        let _ = game.session.apply_player_damage(4);
    }

    if keys.just_pressed(KeyCode::KeyH) {
        game.session.heal_player(2);
    }

    if keys.just_pressed(KeyCode::KeyR) && game.session.player().is_dead {
        game.session.respawn_player();
    }
}

fn handle_debug_weather(
    keys: Res<ButtonInput<KeyCode>>,
    chat_input: Res<ChatInputState>,
    mut lifecycle: ResMut<RuntimeLifecycle>,
    mut lifecycle_hooks: ResMut<RuntimeLifecycleHooks>,
) {
    if chat_input.open {
        return;
    }

    let mut changed = false;

    if keys.just_pressed(KeyCode::F6) {
        changed = lifecycle.controller.set_weather(WeatherKind::Clear) || changed;
    }

    if keys.just_pressed(KeyCode::F7) {
        changed = lifecycle.controller.set_weather(WeatherKind::Rain) || changed;
    }

    if keys.just_pressed(KeyCode::F8) {
        changed = lifecycle.controller.set_weather(WeatherKind::Thunder) || changed;
    }

    if changed {
        consume_runtime_lifecycle_events(&mut lifecycle, &mut lifecycle_hooks);
    }
}

fn handle_block_interaction(
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut game: ResMut<GameState>,
    mut block_destroy_state: ResMut<BlockDestroyState>,
    mut item_in_hand_state: ResMut<ItemInHandAnimationState>,
    mut lifecycle: ResMut<RuntimeLifecycle>,
    look: Res<LookState>,
    cursor_capture: Res<CursorCaptureState>,
    pause_menu: Res<PauseMenuState>,
    mut spawned_chunk_meshes: ResMut<SpawnedChunkMeshes>,
    loaded_chunks: Res<LoadedChunks>,
    block_assets: Res<BlockRenderAssets>,
    runtime_audio: Res<RuntimeAudio>,
    save_root: Res<SaveRoot>,
    perf_config: Res<PerfDebugConfig>,
) {
    if pause_menu.open || !cursor_capture.captured || cursor_capture.just_captured {
        block_destroy_state.clear();
        item_in_hand_state.set_defending(false);
        return;
    }

    let (player_position, player_is_dead, selected_stack) = {
        let player = game.session.player();
        (
            player.position,
            player.is_dead,
            player.inventory.selected_stack(),
        )
    };

    if player_is_dead {
        block_destroy_state.clear();
        item_in_hand_state.set_defending(false);
        return;
    }

    let selected_sword = selected_stack.is_some_and(|stack| is_sword_item(stack.item_id));
    item_in_hand_state.set_defending(selected_sword && mouse.pressed(MouseButton::Right));

    let eye = lce_rust::world::Vec3::new(
        player_position.x,
        player_position.y + CAMERA_EYE_HEIGHT,
        player_position.z,
    );
    let forward = forward_vector_from_yaw_pitch(look.yaw_radians, look.pitch_radians);
    let solid_hit = raycast_first_solid_block(
        &game.blocks,
        eye,
        forward,
        INTERACTION_DISTANCE_BLOCKS as f32,
    );
    let non_air_hit = raycast_first_non_air_block(
        &game.blocks,
        eye,
        forward,
        INTERACTION_DISTANCE_BLOCKS as f32,
    );
    let mut changed = false;
    let mut changed_block = None;
    let mut placed_block_id = None;
    let mut broken_block = None;
    let mut sword_failed_break = None;
    let mut did_swing = false;
    let mut did_place_or_use_item = false;

    let left_mouse_down = mouse.pressed(MouseButton::Left);
    if !left_mouse_down {
        block_destroy_state.clear();
    }

    let left_click_triggered = mouse.just_pressed(MouseButton::Left)
        || (left_mouse_down && item_in_hand_state.can_repeat_left_click_action());

    if mouse.just_pressed(MouseButton::Right)
        && let Some(selected_stack) = selected_stack
    {
        match selected_stack.item_id {
            331 => {
                if let Some(hit) = solid_hit {
                    let target = hit.adjacent_air_block;
                    let support = lce_rust::world::BlockPos::new(target.x, target.y - 1, target.z);
                    let support_block_id = game.blocks.block_id(support);
                    let target_block_id = game.blocks.block_id(target);

                    if target_block_id == 0
                        && support_block_id != 0
                        && !is_fluid_block(support_block_id)
                        && is_solid_block_for_player_collision(support_block_id)
                        && !placement_intersects_player_collider(player_position, target)
                        && apply_block_action(
                            &mut game.blocks,
                            target,
                            BlockAction::Place { block_id: 55 },
                        )
                    {
                        if !game.session.player().allow_flight {
                            let _ = game.session.player_mut().inventory.consume_selected(1);
                        }
                        changed = true;
                        changed_block = Some(target);
                        placed_block_id = Some(55);
                        did_swing = true;
                        did_place_or_use_item = true;
                    }
                }
            }
            325 => {
                if let Some(hit) = non_air_hit {
                    let target_block_id = game.blocks.block_id(hit.block);
                    let filled_bucket_item_id = match target_block_id {
                        WATER_SOURCE_BLOCK_ID => Some(326),
                        LAVA_SOURCE_BLOCK_ID => Some(327),
                        _ => None,
                    };

                    if let Some(filled_bucket_item_id) = filled_bucket_item_id {
                        game.blocks.place_block(hit.block, 0);

                        if !game.session.player().allow_flight {
                            let selected_slot =
                                game.session.player().inventory.selected_hotbar_slot();
                            if selected_stack.count <= 1 {
                                let filled_bucket = ItemStack::new(filled_bucket_item_id, 1).ok();
                                let _ = game
                                    .session
                                    .player_mut()
                                    .inventory
                                    .set(selected_slot, filled_bucket);
                            } else {
                                let inventory = &mut game.session.player_mut().inventory;
                                let _ = inventory.consume_selected(1);
                                inventory.add_item(filled_bucket_item_id, 1);
                            }
                        }

                        changed = true;
                        changed_block = Some(hit.block);
                        did_swing = true;
                        did_place_or_use_item = true;
                    }
                }
            }
            326 | 327 => {
                let mut placement_target = solid_hit.map(|hit| hit.adjacent_air_block);

                if placement_target.is_none()
                    && let Some(hit) = non_air_hit
                    && is_fluid_block(game.blocks.block_id(hit.block))
                {
                    placement_target = Some(hit.block);
                }

                let fluid_block_id = if selected_stack.item_id == 326 {
                    WATER_SOURCE_BLOCK_ID
                } else {
                    LAVA_SOURCE_BLOCK_ID
                };

                if let Some(target) = placement_target
                    && !placement_intersects_player_collider(player_position, target)
                    && apply_block_action(
                        &mut game.blocks,
                        target,
                        BlockAction::Place {
                            block_id: fluid_block_id,
                        },
                    )
                {
                    if !game.session.player().allow_flight {
                        let selected_slot = game.session.player().inventory.selected_hotbar_slot();
                        if selected_stack.count <= 1 {
                            let empty_bucket = ItemStack::new(325, 1).ok();
                            let _ = game
                                .session
                                .player_mut()
                                .inventory
                                .set(selected_slot, empty_bucket);
                        } else {
                            let inventory = &mut game.session.player_mut().inventory;
                            let _ = inventory.consume_selected(1);
                            inventory.add_item(325, 1);
                        }
                    }
                    changed = true;
                    changed_block = Some(target);
                    placed_block_id = Some(fluid_block_id);
                    did_swing = true;
                    did_place_or_use_item = true;
                }
            }
            _ => {
                let block_id = block_id_for_item(selected_stack.item_id);

                if let Some(block_id) = block_id {
                    let mut placement_target = solid_hit.map(|hit| hit.adjacent_air_block);

                    if placement_target.is_none()
                        && let Some(hit) = non_air_hit
                        && is_fluid_block(game.blocks.block_id(hit.block))
                    {
                        placement_target = Some(hit.block);
                    }

                    if let Some(target) = placement_target
                        && !placement_intersects_player_collider(player_position, target)
                        && apply_block_action(
                            &mut game.blocks,
                            target,
                            BlockAction::Place { block_id },
                        )
                    {
                        let placement_face = placement_face_for_target(solid_hit, target);
                        if let Some(data) = block_placement_data(
                            &game.blocks,
                            block_id,
                            target,
                            placement_face,
                            player_position,
                            look.yaw_radians,
                        ) {
                            game.blocks.set_block_data(target, data);
                        }

                        if !game.session.player().allow_flight {
                            let _ = game.session.player_mut().inventory.consume_selected(1);
                        }
                        changed = true;
                        changed_block = Some(target);
                        placed_block_id = Some(block_id);
                        did_swing = true;
                        did_place_or_use_item = true;
                    }
                }
            }
        }
    }

    if left_click_triggered {
        did_swing = true;
    }

    if left_mouse_down {
        if let Some(hit) = solid_hit {
            let previous_block_id = game.blocks.block_id(hit.block);
            let previous_block_aux = u16::from(game.blocks.block_data(hit.block));

            if selected_sword {
                block_destroy_state.clear();
                if left_click_triggered && previous_block_id != 0 {
                    sword_failed_break = Some((previous_block_id, previous_block_aux, hit.block));
                }
            } else if previous_block_id != 0 {
                let creative_break = game.session.player().allow_flight;

                if creative_break {
                    if left_click_triggered
                        && apply_block_action(&mut game.blocks, hit.block, BlockAction::Break)
                    {
                        changed = true;
                        changed_block = Some(hit.block);
                        broken_block = Some((previous_block_id, previous_block_aux));
                    }
                    block_destroy_state.clear();
                } else if left_click_triggered {
                    if block_destroy_state.target != Some(hit.block) {
                        block_destroy_state.target = Some(hit.block);
                        block_destroy_state.progress = 0.0;
                        block_destroy_state.destroy_swings = 0;
                    } else if block_destroy_state.cooldown_swings > 0 {
                        block_destroy_state.cooldown_swings -= 1;
                    } else {
                        let progress_per_tick =
                            block_destroy_progress_per_tick(previous_block_id, selected_stack);
                        if progress_per_tick > 0.0 {
                            block_destroy_state.progress += progress_per_tick * 3.0;
                        }

                        if block_destroy_state.destroy_swings % BLOCK_BREAK_HIT_SOUND_SWING_INTERVAL
                            == 0
                        {
                            let played = play_block_interaction_sound(
                                &mut commands,
                                &runtime_audio,
                                previous_block_id,
                                hit.block.x,
                                hit.block.y,
                                hit.block.z,
                                false,
                            );
                            if !played {
                                play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.20);
                            }
                        }
                        block_destroy_state.destroy_swings =
                            block_destroy_state.destroy_swings.saturating_add(1);

                        if block_destroy_state.progress >= 1.0
                            && apply_block_action(&mut game.blocks, hit.block, BlockAction::Break)
                        {
                            changed = true;
                            changed_block = Some(hit.block);
                            broken_block = Some((previous_block_id, previous_block_aux));
                            block_destroy_state.progress = 0.0;
                            block_destroy_state.destroy_swings = 0;
                            block_destroy_state.cooldown_swings = BLOCK_BREAK_COOLDOWN_SWINGS;
                        }
                    }
                }
            } else {
                block_destroy_state.clear();
            }
        } else {
            block_destroy_state.clear();
        }
    }

    if did_swing {
        item_in_hand_state.swing();
    }

    if did_place_or_use_item {
        item_in_hand_state.item_placed();
    }

    if let Some((block_id, block_aux, target)) = sword_failed_break {
        spawn_block_hit_particles(
            &mut commands,
            &mut meshes,
            &block_assets.opaque_material,
            block_id,
            block_aux,
            target.x,
            target.y,
            target.z,
        );

        let played = play_block_interaction_sound(
            &mut commands,
            &runtime_audio,
            block_id,
            target.x,
            target.y,
            target.z,
            false,
        );
        if !played {
            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.30);
        }
    }

    if changed {
        let Some(target) = changed_block else {
            return;
        };

        for scheduled in fluid_ticks_for_block_change(&game.blocks, target, placed_block_id) {
            lifecycle.controller.schedule_block_tick(
                scheduled.block,
                scheduled.payload_id,
                scheduled.delay_ticks,
            );
        }

        for scheduled in redstone_ticks_for_block_change(&game.blocks, target, placed_block_id) {
            lifecycle.controller.schedule_block_tick(
                scheduled.block,
                scheduled.payload_id,
                scheduled.delay_ticks,
            );
        }

        for chunk in dirty_chunks_for_block(target) {
            if loaded_chunks.0.contains(&chunk) {
                let mesh_start = Instant::now();
                rebuild_chunk_mesh_entity(
                    &mut commands,
                    &mut meshes,
                    &mut spawned_chunk_meshes,
                    &block_assets,
                    &game.blocks,
                    chunk,
                );
                let elapsed = mesh_start.elapsed();
                maybe_log_mesh_rebuild_spike(&perf_config, "player_interaction", chunk, elapsed);
            }
        }

        if let Some((block_id, block_aux)) = broken_block {
            spawn_block_break_particles(
                &mut commands,
                &mut meshes,
                &block_assets.opaque_material,
                block_id,
                block_aux,
                target.x,
                target.y,
                target.z,
            );
        }

        if let Some((block_id, _)) = broken_block {
            let played = play_block_interaction_sound(
                &mut commands,
                &runtime_audio,
                block_id,
                target.x,
                target.y,
                target.z,
                false,
            );
            if !played {
                play_sound(
                    &mut commands,
                    runtime_audio
                        .pop_sfx
                        .as_ref()
                        .or(runtime_audio.click_sfx.as_ref()),
                    0.38,
                );
            }
        } else if let Some(block_id) = placed_block_id {
            let played = play_block_interaction_sound(
                &mut commands,
                &runtime_audio,
                block_id,
                target.x,
                target.y,
                target.z,
                true,
            );
            if !played {
                play_sound(
                    &mut commands,
                    runtime_audio
                        .wood_click_sfx
                        .as_ref()
                        .or(runtime_audio.click_sfx.as_ref()),
                    0.35,
                );
            }
        } else {
            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.35);
        }

        let target_chunk = target_chunk_for_block(target);
        if let Err(error) = game.blocks.save_chunk(&save_root.0, target_chunk) {
            error!("failed to save chunk after block interaction: {error}");
        }

        if let Err(error) = save_world_snapshot(&save_root.0, &game.session.world_snapshot()) {
            error!("failed to save world snapshot after block interaction: {error}");
        }
    }
}

fn sync_block_break_overlay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    game: Res<GameState>,
    block_destroy_state: Res<BlockDestroyState>,
    block_assets: Res<BlockRenderAssets>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    cursor_capture: Res<CursorCaptureState>,
    mut overlay_state: ResMut<BlockBreakOverlayState>,
    mut overlay_query: Query<(Entity, &mut Mesh3d, &mut Visibility), With<BlockBreakOverlay>>,
) {
    let can_show = allow_first_person_view(
        cursor_capture.captured,
        inventory_ui.open,
        pause_menu.open,
        game.session.player().is_dead,
    );

    let Some(target) = block_destroy_state.target else {
        if let Some(entity) = overlay_state.entity
            && let Ok((_, _, mut visibility)) = overlay_query.get_mut(entity)
        {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    if !can_show || block_destroy_state.progress <= 0.0 {
        if let Some(entity) = overlay_state.entity
            && let Ok((_, _, mut visibility)) = overlay_query.get_mut(entity)
        {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let stage = ((block_destroy_state.progress.clamp(0.0, 0.999) * 10.0).floor() as u8).min(9);

    let needs_rebuild = overlay_state.target != Some(target)
        || overlay_state.stage != stage
        || overlay_state.entity.is_none();

    if !needs_rebuild {
        if let Some(entity) = overlay_state.entity
            && let Ok((_, _, mut visibility)) = overlay_query.get_mut(entity)
        {
            *visibility = Visibility::Visible;
            return;
        }
    }

    let Some(mesh_data) = build_block_break_overlay_mesh_data(&game.blocks, target, stage) else {
        if let Some(entity) = overlay_state.entity
            && let Ok((_, _, mut visibility)) = overlay_query.get_mut(entity)
        {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    let rebuilt_mesh = terrain_mesh_to_bevy_mesh(mesh_data);

    if let Some(entity) = overlay_state.entity
        && let Ok((_, mut mesh, mut visibility)) = overlay_query.get_mut(entity)
    {
        if let Some(existing_mesh) = meshes.get_mut(&mesh.0) {
            *existing_mesh = rebuilt_mesh;
        } else {
            mesh.0 = meshes.add(rebuilt_mesh);
        }
        *visibility = Visibility::Visible;
    } else {
        let entity = commands
            .spawn((
                BlockBreakOverlay,
                Mesh3d(meshes.add(rebuilt_mesh)),
                MeshMaterial3d(block_assets.fluid_material.clone()),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        overlay_state.entity = Some(entity);
    }

    overlay_state.target = Some(target);
    overlay_state.stage = stage;
}

fn render_target_block_outline(
    mut gizmos: Gizmos,
    game: Res<GameState>,
    look: Res<LookState>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    cursor_capture: Res<CursorCaptureState>,
) {
    if !allow_first_person_view(
        cursor_capture.captured,
        inventory_ui.open,
        pause_menu.open,
        game.session.player().is_dead,
    ) {
        return;
    }

    let player = game.session.player();
    let eye = lce_rust::world::Vec3::new(
        player.position.x,
        player.position.y + CAMERA_EYE_HEIGHT,
        player.position.z,
    );

    let eye_block = BlockPos::new(
        eye.x.floor() as i32,
        eye.y.floor() as i32,
        eye.z.floor() as i32,
    );
    let eye_block_id = game.blocks.block_id(eye_block);
    if eye_block_id == WATER_SOURCE_BLOCK_ID || eye_block_id == WATER_FLOWING_BLOCK_ID {
        return;
    }

    let forward = forward_vector_from_yaw_pitch(look.yaw_radians, look.pitch_radians);
    let Some(hit) = raycast_first_solid_block(
        &game.blocks,
        eye,
        forward,
        INTERACTION_DISTANCE_BLOCKS as f32,
    ) else {
        return;
    };

    let block_id = game.blocks.block_id(hit.block);
    if block_id == 0 {
        return;
    }

    let bounds = selection_outline_bounds(&game.blocks, hit.block, block_id);
    draw_selection_outline(&mut gizmos, hit.block, bounds);
}

fn selection_outline_bounds(world: &BlockWorld, block: BlockPos, block_id: u16) -> [f32; 6] {
    if matches!(
        block_id,
        50 | REDSTONE_TORCH_OFF_BLOCK_ID | REDSTONE_TORCH_ON_BLOCK_ID
    ) {
        return torch_render_bounds(world.block_data(block) & 0x7);
    }

    if block_id == LEVER_BLOCK_ID {
        return lever_render_bounds(world.block_data(block) & 0x7);
    }

    [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
}

fn torch_render_bounds(data: u8) -> [f32; 6] {
    match data {
        1 => [0.0, 0.2, 0.35, 0.3, 0.8, 0.65],
        2 => [0.7, 0.2, 0.35, 1.0, 0.8, 0.65],
        3 => [0.35, 0.2, 0.0, 0.65, 0.8, 0.3],
        4 => [0.35, 0.2, 0.7, 0.65, 0.8, 1.0],
        _ => [0.4, 0.0, 0.4, 0.6, 0.6, 0.6],
    }
}

fn lever_render_bounds(data: u8) -> [f32; 6] {
    match data {
        1 => [0.0, 0.2, 0.3125, 0.375, 0.8, 0.6875],
        2 => [0.625, 0.2, 0.3125, 1.0, 0.8, 0.6875],
        3 => [0.3125, 0.2, 0.0, 0.6875, 0.8, 0.375],
        4 => [0.3125, 0.2, 0.625, 0.6875, 0.8, 1.0],
        5 | 6 => [0.25, 0.0, 0.25, 0.75, 0.6, 0.75],
        0 | 7 => [0.25, 0.4, 0.25, 0.75, 1.0, 0.75],
        _ => [0.25, 0.0, 0.25, 0.75, 0.6, 0.75],
    }
}

fn draw_selection_outline(gizmos: &mut Gizmos, block: BlockPos, bounds: [f32; 6]) {
    let [bx0, by0, bz0, bx1, by1, bz1] = bounds;
    let x0 = block.x as f32 + bx0 - HIT_OUTLINE_GROW;
    let y0 = block.y as f32 + by0 - HIT_OUTLINE_GROW;
    let z0 = block.z as f32 + bz0 - HIT_OUTLINE_GROW;
    let x1 = block.x as f32 + bx1 + HIT_OUTLINE_GROW;
    let y1 = block.y as f32 + by1 + HIT_OUTLINE_GROW;
    let z1 = block.z as f32 + bz1 + HIT_OUTLINE_GROW;

    let c000 = Vec3::new(x0, y0, z0);
    let c100 = Vec3::new(x1, y0, z0);
    let c010 = Vec3::new(x0, y1, z0);
    let c110 = Vec3::new(x1, y1, z0);
    let c001 = Vec3::new(x0, y0, z1);
    let c101 = Vec3::new(x1, y0, z1);
    let c011 = Vec3::new(x0, y1, z1);
    let c111 = Vec3::new(x1, y1, z1);

    let color = Color::srgba(0.0, 0.0, 0.0, 0.4);
    gizmos.line(c000, c100, color);
    gizmos.line(c100, c101, color);
    gizmos.line(c101, c001, color);
    gizmos.line(c001, c000, color);

    gizmos.line(c010, c110, color);
    gizmos.line(c110, c111, color);
    gizmos.line(c111, c011, color);
    gizmos.line(c011, c010, color);

    gizmos.line(c000, c010, color);
    gizmos.line(c100, c110, color);
    gizmos.line(c101, c111, color);
    gizmos.line(c001, c011, color);
}

fn next_break_particle_random(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32) / (u32::MAX as f32)
}

fn play_block_interaction_sound(
    commands: &mut Commands,
    runtime_audio: &RuntimeAudio,
    block_id: u16,
    block_x: i32,
    block_y: i32,
    block_z: i32,
    is_place: bool,
) -> bool {
    let Some(profile) = legacy_tile_sound_profile_for_block_id(block_id) else {
        return false;
    };

    let event_key = if is_place {
        profile.place_event_key()
    } else {
        profile.break_event_key()
    };
    let volume = (profile.volume() + 1.0) * 0.5;
    let pitch = profile.pitch() * 0.8;

    play_legacy_event_sound(
        commands,
        runtime_audio,
        event_key,
        volume,
        pitch,
        block_interaction_sound_seed(block_x, block_y, block_z, block_id, is_place),
    )
}

fn block_interaction_sound_seed(
    block_x: i32,
    block_y: i32,
    block_z: i32,
    block_id: u16,
    is_place: bool,
) -> u32 {
    (block_x as u32).wrapping_mul(73_856_093)
        ^ (block_y as u32).wrapping_mul(19_349_663)
        ^ (block_z as u32).wrapping_mul(83_492_791)
        ^ (block_id as u32).wrapping_mul(2_654_435_761)
        ^ if is_place { 0x9E37_79B9 } else { 0x85EB_CA6B }
}

fn play_legacy_event_sound(
    commands: &mut Commands,
    runtime_audio: &RuntimeAudio,
    event_key: &str,
    volume: f32,
    pitch: f32,
    variant_seed: u32,
) -> bool {
    let normalized_key = normalize_legacy_event_key(event_key);
    let Some(variants) = runtime_audio.legacy_event_sfx.get(&normalized_key) else {
        return false;
    };

    if variants.is_empty() {
        return false;
    }

    let variant_index = (variant_seed as usize) % variants.len();
    let handle = variants.get(variant_index);
    play_sound_with_pitch(commands, handle, volume, pitch);
    true
}

fn load_legacy_event_audio_handles(
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssetManifest,
) -> HashMap<String, Vec<Handle<AudioSource>>> {
    let Some(relative_dir) = runtime_assets.legacy_event_audio_asset_dir.as_ref() else {
        return HashMap::new();
    };

    let absolute_dir = Path::new("assets").join(relative_dir.replace('/', "\\"));
    if !absolute_dir.exists() {
        return HashMap::new();
    }

    let mut absolute_wav_paths = Vec::new();
    collect_wav_paths_recursive(&absolute_dir, &mut absolute_wav_paths);

    let assets_root = Path::new("assets");
    let mut relative_wav_paths = absolute_wav_paths
        .into_iter()
        .filter_map(|path| {
            let relative = path.strip_prefix(assets_root).ok()?;
            Some(relative.to_string_lossy().replace('\\', "/"))
        })
        .collect::<Vec<_>>();
    relative_wav_paths.sort();

    let mut by_event_key = HashMap::<String, Vec<Handle<AudioSource>>>::new();
    for relative_path in relative_wav_paths {
        let stem = Path::new(&relative_path)
            .file_stem()
            .and_then(|stem| stem.to_str());
        let Some(stem) = stem else {
            continue;
        };

        let Some(event_key) = legacy_event_key_from_file_stem(stem) else {
            continue;
        };

        by_event_key
            .entry(event_key)
            .or_default()
            .push(asset_server.load(relative_path));
    }

    by_event_key
}

fn collect_wav_paths_recursive(directory: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_wav_paths_recursive(&path, output);
            continue;
        }

        let is_wav = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("wav"))
            .unwrap_or(false);
        if is_wav {
            output.push(path);
        }
    }
}

fn legacy_event_key_from_file_stem(file_stem: &str) -> Option<String> {
    let without_bank_index = file_stem.split("__").next().unwrap_or(file_stem);
    let mut normalized = normalize_legacy_event_key(without_bank_index);

    while normalized.ends_with(|ch: char| ch.is_ascii_digit()) {
        normalized.pop();
    }
    while normalized.ends_with('_') {
        normalized.pop();
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_legacy_event_key(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !normalized.is_empty() && !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }

    while normalized.ends_with('_') {
        normalized.pop();
    }

    normalized
}

fn play_sound(commands: &mut Commands, sound: Option<&Handle<AudioSource>>, volume: f32) {
    play_sound_with_pitch(commands, sound, volume, 1.0);
}

fn play_sound_with_pitch(
    commands: &mut Commands,
    sound: Option<&Handle<AudioSource>>,
    volume: f32,
    pitch: f32,
) {
    let Some(sound) = sound else {
        return;
    };

    let speed = pitch.max(0.01);
    let playback = PlaybackSettings {
        speed,
        ..PlaybackSettings::DESPAWN.with_volume(Volume::new(volume))
    };

    commands.spawn((AudioPlayer::new(sound.clone()), playback));
}

fn spawn_block_hit_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    block_id: u16,
    block_aux: u16,
    block_x: i32,
    block_y: i32,
    block_z: i32,
) {
    let (tile_x, tile_y) = terrain_break_particle_tile(block_id, block_aux);
    let mut particle_meshes = vec![None; 16];
    let base_seed = (block_x as u32).wrapping_mul(73_856_093)
        ^ (block_y as u32).wrapping_mul(19_349_663)
        ^ (block_z as u32).wrapping_mul(83_492_791)
        ^ (block_id as u32).wrapping_mul(2_654_435_761)
        ^ 0x9E37_79B9;

    let subdivisions = BREAK_PARTICLE_SUBDIVISIONS as f32;
    for particle_index in 0..BLOCK_HIT_PARTICLE_COUNT {
        let mut seed = base_seed ^ (particle_index as u32).wrapping_mul(1_597_334_677);
        let xx = (next_break_particle_random(&mut seed) * subdivisions)
            .floor()
            .clamp(0.0, subdivisions - 1.0);
        let yy = (next_break_particle_random(&mut seed) * subdivisions)
            .floor()
            .clamp(0.0, subdivisions - 1.0);
        let zz = (next_break_particle_random(&mut seed) * subdivisions)
            .floor()
            .clamp(0.0, subdivisions - 1.0);

        let xp = block_x as f32 + (xx + 0.5) / subdivisions;
        let yp = block_y as f32 + (yy + 0.5) / subdivisions;
        let zp = block_z as f32 + (zz + 0.5) / subdivisions;

        let mut velocity = Vec3::new(
            (next_break_particle_random(&mut seed) * 2.0 - 1.0) * 0.12,
            next_break_particle_random(&mut seed) * 0.10,
            (next_break_particle_random(&mut seed) * 2.0 - 1.0) * 0.12,
        );
        velocity.y += 0.03;

        let speed = (next_break_particle_random(&mut seed) * 0.5 + 0.5) * 0.14;
        let length = velocity.length().max(0.0001);
        velocity = (velocity / length) * speed;

        let size = next_break_particle_random(&mut seed) * 0.35 + 0.45;
        let scale = BREAK_PARTICLE_SCALE_MULTIPLIER * 0.45 * size;
        let lifetime_ticks = 1.5 / (next_break_particle_random(&mut seed) * 0.9 + 0.1);
        let subtile_u = (next_break_particle_random(&mut seed) * 4.0).floor() as u8;
        let subtile_v = (next_break_particle_random(&mut seed) * 4.0).floor() as u8;
        let mesh_index = usize::from(subtile_u.min(3)) * 4 + usize::from(subtile_v.min(3));
        let mesh_handle = particle_meshes[mesh_index]
            .get_or_insert_with(|| {
                meshes.add(build_break_particle_mesh(
                    tile_x,
                    tile_y,
                    subtile_u.min(3),
                    subtile_v.min(3),
                ))
            })
            .clone();

        commands.spawn((
            BreakParticle {
                velocity,
                remaining_lifetime_ticks: lifetime_ticks,
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(Vec3::new(xp, yp, zp)).with_scale(Vec3::splat(scale)),
            Visibility::Visible,
        ));
    }
}

fn spawn_block_break_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    block_id: u16,
    block_aux: u16,
    block_x: i32,
    block_y: i32,
    block_z: i32,
) {
    let (tile_x, tile_y) = terrain_break_particle_tile(block_id, block_aux);
    let mut particle_meshes = vec![None; 16];
    let base_seed = (block_x as u32).wrapping_mul(73_856_093)
        ^ (block_y as u32).wrapping_mul(19_349_663)
        ^ (block_z as u32).wrapping_mul(83_492_791)
        ^ (block_id as u32).wrapping_mul(2_654_435_761);

    let subdivisions = BREAK_PARTICLE_SUBDIVISIONS as usize;
    for xx in 0..subdivisions {
        for yy in 0..subdivisions {
            for zz in 0..subdivisions {
                let particle_index =
                    (xx * subdivisions * subdivisions + yy * subdivisions + zz) as u32;
                let mut seed = base_seed ^ particle_index.wrapping_mul(1_597_334_677);

                let xp = block_x as f32 + (xx as f32 + 0.5) / BREAK_PARTICLE_SUBDIVISIONS as f32;
                let yp = block_y as f32 + (yy as f32 + 0.5) / BREAK_PARTICLE_SUBDIVISIONS as f32;
                let zp = block_z as f32 + (zz as f32 + 0.5) / BREAK_PARTICLE_SUBDIVISIONS as f32;

                let mut velocity = Vec3::new(
                    xp - block_x as f32 - 0.5,
                    yp - block_y as f32 - 0.5,
                    zp - block_z as f32 - 0.5,
                );

                velocity.x += (next_break_particle_random(&mut seed) * 2.0 - 1.0) * 0.4;
                velocity.y += (next_break_particle_random(&mut seed) * 2.0 - 1.0) * 0.4;
                velocity.z += (next_break_particle_random(&mut seed) * 2.0 - 1.0) * 0.4;

                let speed = (next_break_particle_random(&mut seed)
                    + next_break_particle_random(&mut seed)
                    + 1.0)
                    * 0.15;
                let length = velocity.length().max(0.0001);
                velocity = (velocity / length) * speed * 0.4;
                velocity.y += 0.1;

                let size = next_break_particle_random(&mut seed) * 0.5 + 0.5;
                let scale = BREAK_PARTICLE_SCALE_MULTIPLIER * size;
                let lifetime_ticks = 4.0 / (next_break_particle_random(&mut seed) * 0.9 + 0.1);
                let subtile_u = (next_break_particle_random(&mut seed) * 4.0).floor() as u8;
                let subtile_v = (next_break_particle_random(&mut seed) * 4.0).floor() as u8;
                let mesh_index = usize::from(subtile_u.min(3)) * 4 + usize::from(subtile_v.min(3));
                let mesh_handle = particle_meshes[mesh_index]
                    .get_or_insert_with(|| {
                        meshes.add(build_break_particle_mesh(
                            tile_x,
                            tile_y,
                            subtile_u.min(3),
                            subtile_v.min(3),
                        ))
                    })
                    .clone();

                commands.spawn((
                    BreakParticle {
                        velocity,
                        remaining_lifetime_ticks: lifetime_ticks,
                    },
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(Vec3::new(xp, yp, zp))
                        .with_scale(Vec3::splat(scale)),
                    Visibility::Visible,
                ));
            }
        }
    }
}

fn tick_break_particles(
    time: Res<Time>,
    pause_menu: Res<PauseMenuState>,
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut BreakParticle)>,
) {
    if pause_menu.open {
        return;
    }

    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    let tick_delta = dt * 20.0;

    for (entity, mut transform, mut particle) in &mut particle_query {
        particle.remaining_lifetime_ticks -= tick_delta;
        if particle.remaining_lifetime_ticks <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        particle.velocity.y -= BREAK_PARTICLE_TICK_GRAVITY * tick_delta;
        let drag = BREAK_PARTICLE_TICK_DRAG.powf(tick_delta);
        particle.velocity *= drag;
        transform.translation += particle.velocity * tick_delta;
    }
}

fn handle_inventory_crafting(
    inventory_ui: Res<InventoryUiState>,
    creative_ui: Res<CreativeInventoryState>,
    mut commands: Commands,
    runtime_audio: Res<RuntimeAudio>,
    mut game: ResMut<GameState>,
    mut interaction_query: Query<
        (&Interaction, &CraftingRecipeButtonUi),
        (Changed<Interaction>, With<Button>),
    >,
    mut status_query: Query<(&mut Text, &mut TextColor), With<CraftingStatusTextUi>>,
) {
    if !inventory_ui.open
        || (game.session.player().allow_flight && !creative_ui.show_player_inventory_tab)
    {
        return;
    }

    for (interaction, recipe_button) in &mut interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.28);

        let outcome = craft_recipe(
            &mut game.session.player_mut().inventory,
            recipe_button.recipe_id,
            1,
        );

        let (message, color) = if outcome.crafted_times > 0 {
            let recipe_title = crafting_recipe_title(recipe_button.recipe_id);
            let recipe = recipe_by_id(recipe_button.recipe_id)
                .expect("crafting button should reference a valid recipe");
            let count_label = crafting_recipe_count_label(recipe.output_count);
            let crafted_message = if count_label.is_empty() {
                format!("Crafted {recipe_title}")
            } else {
                format!("Crafted {recipe_title} {count_label}")
            };
            (crafted_message, Color::srgb(0.72, 0.90, 0.64))
        } else {
            (
                "Missing ingredients or no space".to_string(),
                Color::srgb(0.95, 0.62, 0.62),
            )
        };

        for (mut status_text, mut status_color) in &mut status_query {
            status_text.0 = message.clone();
            status_color.0 = color;
        }
    }
}

fn handle_creative_inventory_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    mut creative_ui: ResMut<CreativeInventoryState>,
) {
    let mut wheel_delta = 0.0_f32;
    for event in mouse_wheel_events.read() {
        wheel_delta += event.y;
    }

    if !inventory_ui.open || !game.session.player().allow_flight {
        return;
    }

    if keys.just_pressed(KeyCode::KeyI) {
        creative_ui.show_player_inventory_tab = !creative_ui.show_player_inventory_tab;
        return;
    }

    if creative_ui.show_player_inventory_tab {
        return;
    }

    if keys.just_pressed(KeyCode::ArrowRight) {
        creative_ui.tab = creative_ui.tab.next();
    }

    if keys.just_pressed(KeyCode::ArrowLeft) {
        creative_ui.tab = creative_ui.tab.previous();
    }

    let shift_held = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let dynamic_group_count = creative_tab_dynamic_group_count(creative_ui.tab);
    let page_up_pressed = keys.just_pressed(KeyCode::PageUp) || keys.just_pressed(KeyCode::ArrowUp);
    let page_down_pressed =
        keys.just_pressed(KeyCode::PageDown) || keys.just_pressed(KeyCode::ArrowDown);

    if page_up_pressed {
        if creative_ui.tab == CreativeInventoryTab::Brewing
            && dynamic_group_count > 0
            && !shift_held
            && keys.just_pressed(KeyCode::PageUp)
        {
            let next_dynamic_group =
                creative_next_dynamic_group(creative_ui.tab, creative_ui.active_dynamic_group());
            creative_ui.set_active_dynamic_group(next_dynamic_group);
            creative_ui.set_active_page(0);
        } else {
            let previous_page = creative_ui.active_page().saturating_sub(1);
            creative_ui.set_active_page(previous_page);
        }
    }

    let page_count = creative_tab_entry_page_count_for_dynamic_group(
        creative_ui.tab,
        creative_ui.active_dynamic_group(),
    );
    if page_down_pressed {
        let next_page = (creative_ui.active_page() + 1).min(page_count.saturating_sub(1));
        creative_ui.set_active_page(next_page);
    }

    let wheel_steps = if wheel_delta > 0.0 {
        wheel_delta.ceil() as i32
    } else if wheel_delta < 0.0 {
        wheel_delta.floor() as i32
    } else {
        0
    };
    if wheel_steps > 0 {
        let previous_page = creative_ui
            .active_page()
            .saturating_sub(wheel_steps as usize);
        creative_ui.set_active_page(previous_page);
    } else if wheel_steps < 0 {
        let next_page = (creative_ui.active_page() + ((-wheel_steps) as usize))
            .min(page_count.saturating_sub(1));
        creative_ui.set_active_page(next_page);
    }

    let clamped_page = creative_ui.active_page().min(page_count.saturating_sub(1));
    creative_ui.set_active_page(clamped_page);
}

fn handle_creative_inventory_clicks(
    inventory_ui: Res<InventoryUiState>,
    mut creative_ui: ResMut<CreativeInventoryState>,
    mut commands: Commands,
    runtime_audio: Res<RuntimeAudio>,
    mut game: ResMut<GameState>,
    player_tab_interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<CreativeInventoryPlayerTabButtonUi>,
        ),
    >,
    mut tab_interactions: Query<
        (&Interaction, &CreativeTabButtonUi),
        (
            Changed<Interaction>,
            With<Button>,
            Without<CreativeSelectorSlotButtonUi>,
        ),
    >,
    mut selector_interactions: Query<
        (&Interaction, &CreativeSelectorSlotButtonUi),
        (
            Changed<Interaction>,
            With<Button>,
            Without<CreativeTabButtonUi>,
        ),
    >,
    mut hotbar_interactions: Query<
        (&Interaction, &CreativeHotbarSlotUi),
        (
            Changed<Interaction>,
            With<Button>,
            Without<CreativeTabButtonUi>,
            Without<CreativeSelectorSlotButtonUi>,
        ),
    >,
) {
    if !inventory_ui.open || !game.session.player().allow_flight {
        return;
    }

    for interaction in &player_tab_interactions {
        if *interaction == Interaction::Pressed {
            creative_ui.show_player_inventory_tab = true;
            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.28);
        }
    }

    if creative_ui.show_player_inventory_tab {
        return;
    }

    for (interaction, tab_button) in &mut tab_interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        creative_ui.show_player_inventory_tab = false;
        creative_ui.tab = tab_button.tab;
        play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.28);
    }

    let page_count = creative_tab_entry_page_count_for_dynamic_group(
        creative_ui.tab,
        creative_ui.active_dynamic_group(),
    );
    let clamped_page = creative_ui.active_page().min(page_count.saturating_sub(1));
    creative_ui.set_active_page(clamped_page);
    let selector_items = creative_selector_entries_page_for_dynamic_group(
        creative_ui.tab,
        creative_ui.active_dynamic_group(),
        clamped_page,
    );

    for (interaction, slot_button) in &mut selector_interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(entry) = selector_items[slot_button.slot] else {
            continue;
        };

        let Some(target_slot) =
            place_creative_entry_in_hotbar(&mut game.session.player_mut().inventory, entry, 64)
        else {
            continue;
        };

        let _ = game
            .session
            .player_mut()
            .inventory
            .select_hotbar_slot(target_slot);
        play_sound(
            &mut commands,
            runtime_audio
                .pop_sfx
                .as_ref()
                .or(runtime_audio.click_sfx.as_ref()),
            0.30,
        );
    }

    for (interaction, hotbar_slot) in &mut hotbar_interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let slot = hotbar_slot.slot;
        let had_stack = game
            .session
            .player()
            .inventory
            .get(slot)
            .ok()
            .flatten()
            .is_some();
        let _ = game.session.player_mut().inventory.select_hotbar_slot(slot);

        if had_stack {
            let _ = game.session.player_mut().inventory.set(slot, None);
            play_sound(
                &mut commands,
                runtime_audio
                    .pop_sfx
                    .as_ref()
                    .or(runtime_audio.click_sfx.as_ref()),
                0.30,
            );
        } else {
            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.28);
        }
    }
}

fn handle_inventory_slot_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    inventory_ui: Res<InventoryUiState>,
    creative_ui: Res<CreativeInventoryState>,
    pause_menu: Res<PauseMenuState>,
    mut drag_state: ResMut<InventoryDragState>,
    mut commands: Commands,
    runtime_audio: Res<RuntimeAudio>,
    mut game: ResMut<GameState>,
    mut slot_interactions: Query<
        (&Interaction, &InventorySlotUi),
        (With<Button>, Without<CraftingRecipeButtonUi>),
    >,
) {
    let show_inventory = inventory_ui.open
        && (!game.session.player().allow_flight || creative_ui.show_player_inventory_tab)
        && !pause_menu.open
        && !game.session.player().is_dead;

    if !show_inventory {
        if let Some(held) = drag_state.held_stack
            && let Some(source_slot) = drag_state.source_slot
        {
            let destination_empty = game
                .session
                .player()
                .inventory
                .get(source_slot)
                .ok()
                .flatten()
                .is_none();
            if destination_empty {
                let _ = game
                    .session
                    .player_mut()
                    .inventory
                    .set(source_slot, Some(held));
            } else {
                let _ = game.session.player_mut().inventory.add_item_with_aux(
                    held.item_id,
                    held.aux,
                    u32::from(held.count),
                );
            }
        }
        drag_state.clear();
        return;
    }

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    for (interaction, slot_ui) in &mut slot_interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let slot = slot_ui.slot;
        let slot_stack = game
            .session
            .player()
            .inventory
            .get(slot)
            .expect("inventory slot index should be valid");

        if let Some(held_stack) = drag_state.held_stack {
            let source_hotbar_slot = drag_state
                .source_slot
                .is_some_and(|source| source < HOTBAR_SLOTS);
            if game.session.player().allow_flight && source_hotbar_slot && slot >= HOTBAR_SLOTS {
                drag_state.clear();
                play_sound(
                    &mut commands,
                    runtime_audio
                        .pop_sfx
                        .as_ref()
                        .or(runtime_audio.click_sfx.as_ref()),
                    0.30,
                );
                continue;
            }

            if slot_stack.is_none() {
                let _ = game
                    .session
                    .player_mut()
                    .inventory
                    .set(slot, Some(held_stack));
                drag_state.clear();
            } else {
                let _ = game
                    .session
                    .player_mut()
                    .inventory
                    .set(slot, Some(held_stack));
                drag_state.held_stack = slot_stack;
                drag_state.source_slot = Some(slot);
            }

            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.28);
            continue;
        }

        let Some(slot_stack) = slot_stack else {
            continue;
        };

        let _ = game.session.player_mut().inventory.set(slot, None);
        drag_state.held_stack = Some(slot_stack);
        drag_state.source_slot = Some(slot);
        play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.26);
    }
}

fn handle_pause_and_death_buttons(
    mut game: ResMut<GameState>,
    save_root: Res<SaveRoot>,
    mut inventory_ui: ResMut<InventoryUiState>,
    mut pause_menu: ResMut<PauseMenuState>,
    mut capture_state: ResMut<CursorCaptureState>,
    mut commands: Commands,
    runtime_audio: Res<RuntimeAudio>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut exit_events: EventWriter<AppExit>,
    resume_interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<PauseResumeButtonUi>,
            Without<PauseSaveQuitButtonUi>,
        ),
    >,
    pause_quit_interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<PauseSaveQuitButtonUi>,
            Without<PauseResumeButtonUi>,
        ),
    >,
    pause_misc_interactions: Query<
        (
            &Interaction,
            Option<&PauseHelpOptionsButtonUi>,
            Option<&PauseLeaderboardsButtonUi>,
            Option<&PauseAchievementsButtonUi>,
            Option<&PauseExitButtonUi>,
        ),
        (
            Changed<Interaction>,
            With<Button>,
            Without<PauseResumeButtonUi>,
            Without<PauseSaveQuitButtonUi>,
        ),
    >,
    respawn_interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<DeathRespawnButtonUi>,
            Without<DeathQuitButtonUi>,
        ),
    >,
    death_quit_interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<DeathQuitButtonUi>,
            Without<DeathRespawnButtonUi>,
        ),
    >,
) {
    for interaction in &resume_interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        pause_menu.open = false;
        inventory_ui.open = false;
        play_sound(
            &mut commands,
            runtime_audio
                .back_sfx
                .as_ref()
                .or(runtime_audio.click_sfx.as_ref()),
            0.30,
        );
        if let Ok(mut window) = windows.get_single_mut() {
            capture_state.captured = capture_cursor(&mut window);
            capture_state.just_captured = capture_state.captured;
        }
    }

    for interaction in &pause_quit_interactions {
        if *interaction == Interaction::Pressed {
            if let Err(error) = game.blocks.save_all_touched_chunks(&save_root.0) {
                error!("failed to save touched chunks from pause menu: {error}");
            }

            if let Err(error) = save_world_snapshot(&save_root.0, &game.session.world_snapshot()) {
                error!("failed to save world snapshot from pause menu: {error}");
            }

            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.30);
        }
    }

    for (interaction, help, leaderboards, achievements, exit) in &pause_misc_interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if help.is_some() || leaderboards.is_some() || achievements.is_some() {
            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.30);
        } else if exit.is_some() {
            play_sound(
                &mut commands,
                runtime_audio
                    .back_sfx
                    .as_ref()
                    .or(runtime_audio.click_sfx.as_ref()),
                0.34,
            );
            exit_events.send(AppExit::Success);
        }
    }

    for interaction in &respawn_interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if game.session.player().is_dead {
            game.session.respawn_player();
            pause_menu.open = false;
            inventory_ui.open = false;
            play_sound(&mut commands, runtime_audio.click_sfx.as_ref(), 0.30);
            if let Ok(mut window) = windows.get_single_mut() {
                capture_state.captured = capture_cursor(&mut window);
                capture_state.just_captured = capture_state.captured;
            }
        }
    }

    for interaction in &death_quit_interactions {
        if *interaction == Interaction::Pressed {
            play_sound(
                &mut commands,
                runtime_audio
                    .back_sfx
                    .as_ref()
                    .or(runtime_audio.click_sfx.as_ref()),
                0.34,
            );
            exit_events.send(AppExit::Success);
        }
    }
}

fn sync_pause_and_death_ui(
    game: Res<GameState>,
    mut inventory_ui: ResMut<InventoryUiState>,
    mut pause_menu: ResMut<PauseMenuState>,
    mut capture_state: ResMut<CursorCaptureState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut pause_root_query: Query<&mut Node, (With<PauseMenuRootUi>, Without<DeathScreenRootUi>)>,
    mut death_root_query: Query<&mut Node, (With<DeathScreenRootUi>, Without<PauseMenuRootUi>)>,
) {
    let is_dead = game.session.player().is_dead;
    let pause_visible = show_pause_menu(pause_menu.open, is_dead);
    let death_visible = show_death_screen(is_dead);

    if is_dead {
        pause_menu.open = false;
        inventory_ui.open = false;

        if let Ok(mut window) = windows.get_single_mut() {
            release_cursor(&mut window);
            capture_state.captured = false;
            capture_state.just_captured = false;
        }
    }

    for mut root_node in &mut pause_root_query {
        root_node.display = if pause_visible {
            Display::Flex
        } else {
            Display::None
        };
    }

    for mut root_node in &mut death_root_query {
        root_node.display = if death_visible {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn persist_on_exit(
    mut exit_events: EventReader<AppExit>,
    game: Res<GameState>,
    save_root: Res<SaveRoot>,
) {
    let mut should_persist = false;
    for _ in exit_events.read() {
        should_persist = true;
    }

    if !should_persist {
        return;
    }

    if let Err(error) = game.blocks.save_all_touched_chunks(&save_root.0) {
        error!("failed to save touched chunks at shutdown: {error}");
    }

    if let Err(error) = save_world_snapshot(&save_root.0, &game.session.world_snapshot()) {
        error!("failed to save world snapshot at shutdown: {error}");
    }
}

fn apply_generated_chunk(world: &mut BlockWorld, generated: GeneratedChunk) {
    world.replace_chunk_blocks(generated.chunk, generated.blocks);
}

fn apply_lifecycle_runtime_hooks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut clear_color: ResMut<ClearColor>,
    mut game: ResMut<GameState>,
    mut lifecycle: ResMut<RuntimeLifecycle>,
    loaded_chunks: Res<LoadedChunks>,
    block_assets: Res<BlockRenderAssets>,
    mut spawned_chunk_meshes: ResMut<SpawnedChunkMeshes>,
    mut lifecycle_hooks: ResMut<RuntimeLifecycleHooks>,
    perf_config: Res<PerfDebugConfig>,
    mut perf_state: ResMut<PerfDebugState>,
) {
    let lifecycle_start = Instant::now();
    let mut perf_stats = LifecyclePerfStats::default();

    let (r, g, b) = sky_color_from_brightness(lifecycle_hooks.environment.sky_brightness);
    clear_color.0 = Color::srgb(r, g, b);

    let process_ticks_start = Instant::now();
    let triggered_block_ticks = std::mem::take(&mut lifecycle_hooks.triggered_block_ticks);
    perf_stats.triggered_block_ticks = triggered_block_ticks.len();

    for tick in triggered_block_ticks {
        if let Some(outcome) = process_scheduled_fluid_tick(&mut game.blocks, tick) {
            perf_stats.fluid_tick_outcomes += 1;
            perf_stats.fluid_changed_chunks += outcome.changed_chunks.len();
            perf_stats.fluid_changed_blocks += outcome.changed_blocks.len();
            perf_stats.fluid_rescheduled_ticks += outcome.scheduled_ticks.len();

            lifecycle_hooks
                .pending_relight_chunks
                .extend(outcome.changed_chunks);

            for scheduled in outcome.scheduled_ticks {
                lifecycle.controller.schedule_block_tick(
                    scheduled.block,
                    scheduled.payload_id,
                    scheduled.delay_ticks,
                );
            }
        }

        if let Some(outcome) = process_scheduled_redstone_tick(&mut game.blocks, tick) {
            perf_stats.redstone_tick_outcomes += 1;
            perf_stats.redstone_changed_chunks += outcome.changed_chunks.len();
            perf_stats.redstone_rescheduled_ticks += outcome.scheduled_ticks.len();

            lifecycle_hooks
                .pending_relight_chunks
                .extend(outcome.changed_chunks);

            for scheduled in outcome.scheduled_ticks {
                lifecycle.controller.schedule_block_tick(
                    scheduled.block,
                    scheduled.payload_id,
                    scheduled.delay_ticks,
                );
            }
        }
    }
    perf_stats.process_ticks = process_ticks_start.elapsed();

    let triggered_tile_ticks = std::mem::take(&mut lifecycle_hooks.triggered_tile_ticks);
    perf_stats.triggered_tile_ticks = triggered_tile_ticks.len();

    let relight_chunks: Vec<_> = lifecycle_hooks
        .pending_relight_chunks
        .iter()
        .copied()
        .collect();
    perf_stats.relight_chunks_requested = relight_chunks.len();
    lifecycle_hooks.pending_relight_chunks.clear();

    for chunk in relight_chunks {
        if !loaded_chunks.0.contains(&chunk) {
            continue;
        }

        let mesh_start = Instant::now();
        rebuild_chunk_mesh_entity(
            &mut commands,
            &mut meshes,
            &mut spawned_chunk_meshes,
            &block_assets,
            &game.blocks,
            chunk,
        );
        let elapsed = mesh_start.elapsed();
        perf_stats.mesh_rebuild += elapsed;
        perf_stats.relight_chunks_rebuilt += 1;
        maybe_log_mesh_rebuild_spike(&perf_config, "lifecycle_relight", chunk, elapsed);
    }

    perf_stats.total = lifecycle_start.elapsed();
    maybe_log_lifecycle_perf(&perf_config, &mut perf_state, &perf_stats);
}

fn consume_runtime_lifecycle_events(
    lifecycle: &mut RuntimeLifecycle,
    lifecycle_hooks: &mut RuntimeLifecycleHooks,
) {
    let events = lifecycle.controller.drain_events();
    if events.is_empty() {
        return;
    }

    let batch = consume_lifecycle_events(&mut lifecycle_hooks.environment, &events);
    lifecycle_hooks
        .pending_relight_chunks
        .extend(batch.relight_chunks);
    lifecycle_hooks
        .triggered_block_ticks
        .extend(batch.triggered_block_ticks);
    lifecycle_hooks
        .triggered_tile_ticks
        .extend(batch.triggered_tile_ticks);
}

fn rebuild_chunk_mesh_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    spawned_chunk_meshes: &mut SpawnedChunkMeshes,
    block_assets: &BlockRenderAssets,
    world: &BlockWorld,
    chunk: ChunkPos,
) {
    if let Some(entity) = spawned_chunk_meshes.0.remove(&chunk) {
        commands.entity(entity).despawn_recursive();
    }

    let Some(mesh_data) = build_chunk_mesh_data(world, chunk) else {
        return;
    };

    let (opaque_mesh, fluid_mesh) = split_terrain_mesh_by_alpha(mesh_data);

    if opaque_mesh.indices.is_empty() && fluid_mesh.indices.is_empty() {
        return;
    }

    let entity = commands
        .spawn((Transform::default(), Visibility::default()))
        .id();

    commands.entity(entity).with_children(|parent| {
        if !opaque_mesh.indices.is_empty() {
            let mesh_handle = meshes.add(terrain_mesh_to_bevy_mesh(opaque_mesh));
            parent.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(block_assets.opaque_material.clone()),
                Transform::default(),
            ));
        }

        if !fluid_mesh.indices.is_empty() {
            let mesh_handle = meshes.add(terrain_mesh_to_bevy_mesh(fluid_mesh));
            parent.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(block_assets.fluid_material.clone()),
                Transform::default(),
            ));
        }
    });

    spawned_chunk_meshes.0.insert(chunk, entity);
}

fn split_terrain_mesh_by_alpha(mesh_data: TerrainMeshData) -> (TerrainMeshData, TerrainMeshData) {
    let face_count = mesh_data.positions.len() / 4;
    let mut opaque = TerrainMeshData::default();
    let mut fluid = TerrainMeshData::default();

    for face_index in 0..face_count {
        let vertex_start = face_index * 4;
        let alpha = mesh_data
            .colors
            .get(vertex_start)
            .map(|color| color[3])
            .unwrap_or(1.0);
        let is_fluid_face = mesh_data
            .face_is_fluid
            .get(face_index)
            .copied()
            .unwrap_or(alpha < 0.999);
        let target = if is_fluid_face {
            &mut fluid
        } else {
            &mut opaque
        };

        let base = u32::try_from(target.positions.len()).unwrap_or(u32::MAX - 4);
        target
            .positions
            .extend_from_slice(&mesh_data.positions[vertex_start..vertex_start + 4]);
        target
            .normals
            .extend_from_slice(&mesh_data.normals[vertex_start..vertex_start + 4]);
        target
            .uvs
            .extend_from_slice(&mesh_data.uvs[vertex_start..vertex_start + 4]);
        target
            .colors
            .extend_from_slice(&mesh_data.colors[vertex_start..vertex_start + 4]);
        target
            .indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        target.face_is_fluid.push(is_fluid_face);
    }

    (opaque, fluid)
}

fn terrain_mesh_to_bevy_mesh(mesh_data: TerrainMeshData) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh_data.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mesh_data.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, mesh_data.uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, mesh_data.colors);
    mesh.insert_indices(Indices::U32(mesh_data.indices));

    mesh
}

fn bevy_forward_from_look(look: &LookState) -> Vec3 {
    let forward = forward_vector_from_yaw_pitch(look.yaw_radians, look.pitch_radians);
    Vec3::new(forward.x, forward.y, forward.z)
}

fn setup_ui_overlay(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 2,
            ..default()
        },
    ));

    commands.spawn((
        FpsTextUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Px(8.0),
            ..default()
        },
        Text::new("FPS: --"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));

    commands
        .spawn((
            ChatRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                right: Val::Px(12.0),
                bottom: Val::Px(12.0),
                height: Val::Px(30.0),
                display: Display::None,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
        ))
        .with_children(|root| {
            root.spawn((
                ChatInputTextUi,
                Text::new("> _"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });

    if let Some(icons_texture_path) = runtime_assets.0.icons_texture_asset_path.as_ref() {
        commands
            .spawn((
                CrosshairRootUi,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Px(15.0),
                        height: Val::Px(15.0),
                        ..default()
                    },
                    ImageNode::new(asset_server.load(icons_texture_path.clone()))
                        .with_rect(Rect::new(0.0, 0.0, 15.0, 15.0)),
                ));
            });
    }

    let first_person_icon_texture = runtime_assets
        .0
        .items_texture_asset_path
        .as_ref()
        .or(runtime_assets.0.terrain_texture_asset_path.as_ref())
        .map(|path| asset_server.load(path.clone()));
    let first_person_icon_node = if let Some(texture) = first_person_icon_texture {
        ImageNode::new(texture).with_rect(Rect::new(0.0, 0.0, 16.0, 16.0))
    } else {
        ImageNode::default()
    };

    commands.spawn((
        FirstPersonHeldItemIconUi,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(120.0),
            bottom: Val::Px(72.0),
            width: Val::Px(96.0),
            height: Val::Px(96.0),
            ..default()
        },
        first_person_icon_node,
        Visibility::Hidden,
    ));

    spawn_first_person_item_camera(commands);
    spawn_ui_item_overlay_camera(commands);

    spawn_hotbar_ui(commands, asset_server, runtime_assets);
    spawn_health_hud(commands, asset_server, runtime_assets);
    spawn_hunger_hud(commands, asset_server, runtime_assets);
    spawn_xp_hud(commands, asset_server, runtime_assets);
    spawn_inventory_ui(commands, asset_server, runtime_assets);
    spawn_creative_inventory_ui(commands, asset_server, runtime_assets);
    spawn_pause_menu_ui(commands, asset_server, runtime_assets);
    spawn_death_screen_ui(commands, asset_server, runtime_assets);
}

fn spawn_ui_item_overlay_camera(commands: &mut Commands) {
    commands.spawn((
        UiItemCamera,
        Camera3d::default(),
        Camera {
            order: 3,
            clear_color: ClearColorConfig::None,
            hdr: false,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::WindowSize,
            near: -1000.0,
            far: 1000.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0.0, 0.0, 400.0),
        RenderLayers::layer(UI_ITEM_RENDER_LAYER),
    ));
}

fn spawn_first_person_item_camera(commands: &mut Commands) {
    commands.spawn((
        FirstPersonItemCamera,
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            hdr: false,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: FIRST_PERSON_ITEM_FOV_DEGREES.to_radians(),
            near: 0.05,
            ..default()
        }),
        Transform::default(),
        RenderLayers::layer(FIRST_PERSON_ITEM_RENDER_LAYER),
    ));
}

fn legacy_button_rect(y_image: u8) -> Rect {
    let y = 46.0 + (f32::from(y_image) * 20.0);
    Rect::new(0.0, y, 200.0, y + 20.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiIconAtlas {
    Terrain,
    Items,
}

fn terrain_icon_rect_for_block(block_id: u16) -> Rect {
    let (tile_x, tile_y) = terrain_icon_tile_for_block(block_id);
    let tile_size = 16.0;
    let left = f32::from(tile_x) * tile_size;
    let top = f32::from(tile_y) * tile_size;
    Rect::new(left, top, left + tile_size, top + tile_size)
}

fn terrain_icon_rect_for_block_with_aux(block_id: u16, aux: u16) -> Rect {
    let (tile_x, tile_y) = terrain_icon_tile_for_block_with_aux(block_id, aux);
    let tile_size = 16.0;
    let left = f32::from(tile_x) * tile_size;
    let top = f32::from(tile_y) * tile_size;
    Rect::new(left, top, left + tile_size, top + tile_size)
}

fn terrain_icon_tile_for_block(block_id: u16) -> (u8, u8) {
    let explicit = atlas_tile_for_block_face(block_id, BlockFace::Top);
    if explicit != (1, 0) || block_id == 1 {
        return explicit;
    }

    if block_id <= 255 {
        return ((block_id % 16) as u8, (block_id / 16) as u8);
    }

    (1, 0)
}

fn terrain_icon_tile_for_block_with_aux(block_id: u16, aux: u16) -> (u8, u8) {
    match block_id {
        5 => (4, 0),
        6 => match aux.min(3) {
            0 => (15, 0),
            1 => (15, 3),
            2 => (15, 4),
            3 => (14, 1),
            _ => (15, 0),
        },
        17 => match aux.min(3) {
            0 => (4, 1),
            1 => (4, 7),
            2 => (5, 7),
            3 => (9, 9),
            _ => (4, 1),
        },
        18 => match aux.min(3) {
            0 => (4, 3),
            1 => (4, 8),
            2 => (4, 3),
            3 => (4, 12),
            _ => (4, 3),
        },
        24 => match aux.min(2) {
            0 => (0, 12),
            1 => (6, 14),
            2 => (5, 14),
            _ => (0, 12),
        },
        31 => match aux.min(2) {
            0 => (7, 3),
            1 => (7, 2),
            2 => (8, 3),
            _ => (7, 2),
        },
        35 | 171 => match aux.min(15) {
            0 => (0, 4),
            1 => (2, 13),
            2 => (2, 12),
            3 => (2, 11),
            4 => (2, 10),
            5 => (2, 9),
            6 => (2, 8),
            7 => (2, 7),
            8 => (1, 14),
            9 => (1, 13),
            10 => (1, 12),
            11 => (1, 11),
            12 => (1, 10),
            13 => (1, 9),
            14 => (1, 8),
            15 => (1, 7),
            _ => (0, 4),
        },
        97 => match aux.min(2) {
            0 => (1, 0),
            1 => (0, 1),
            2 => (6, 3),
            _ => (1, 0),
        },
        98 => match aux.min(3) {
            0 => (6, 3),
            1 => (4, 6),
            2 => (5, 6),
            3 => (5, 13),
            _ => (6, 3),
        },
        126 => (4, 0),
        139 => match aux.min(1) {
            0 => (0, 1),
            1 => (4, 2),
            _ => (0, 1),
        },
        155 => match aux.min(2) {
            0 => (11, 14),
            1 => (9, 14),
            2 => (10, 14),
            _ => (11, 14),
        },
        _ => terrain_icon_tile_for_block(block_id),
    }
}

fn items_icon_rect_for_index(icon_index: u16) -> Rect {
    let tile_size = 16.0;
    let tile_x = icon_index % 16;
    let tile_y = icon_index / 16;
    let left = f32::from(tile_x) * tile_size;
    let top = f32::from(tile_y) * tile_size;
    Rect::new(left, top, left + tile_size, top + tile_size)
}

fn record_icon_index(item_id: u16) -> Option<u16> {
    match item_id {
        2256 => Some(240),
        2257 => Some(241),
        2258 => Some(242),
        2259 => Some(243),
        2260 => Some(244),
        2261 => Some(245),
        2262 => Some(246),
        2263 => Some(247),
        2264 => Some(248),
        2265 => Some(249),
        2266 => Some(250),
        2267 => Some(251),
        _ => None,
    }
}

fn items_icon_tile_for_item(item_id: u16) -> Option<(u8, u8)> {
    match item_id {
        256 => Some((2, 5)),
        257 => Some((2, 6)),
        258 => Some((2, 7)),
        259 => Some((5, 0)),
        260 => Some((10, 0)),
        261 => Some((5, 1)),
        262 => Some((5, 2)),
        263 => Some((7, 0)),
        264 => Some((7, 3)),
        265 => Some((7, 1)),
        266 => Some((7, 2)),
        267 => Some((2, 4)),
        268 => Some((0, 4)),
        269 => Some((0, 5)),
        270 => Some((0, 6)),
        271 => Some((0, 7)),
        272 => Some((1, 4)),
        273 => Some((1, 5)),
        274 => Some((1, 6)),
        275 => Some((1, 7)),
        276 => Some((3, 4)),
        277 => Some((3, 5)),
        278 => Some((3, 6)),
        279 => Some((3, 7)),
        280 => Some((5, 3)),
        281 => Some((7, 4)),
        282 => Some((8, 4)),
        283 => Some((4, 4)),
        284 => Some((4, 5)),
        285 => Some((4, 6)),
        286 => Some((4, 7)),
        287 => Some((8, 0)),
        288 => Some((8, 1)),
        289 => Some((8, 2)),
        290 => Some((0, 8)),
        291 => Some((1, 8)),
        292 => Some((2, 8)),
        293 => Some((3, 8)),
        294 => Some((4, 8)),
        295 => Some((9, 0)),
        296 => Some((9, 1)),
        297 => Some((9, 2)),
        298 => Some((0, 0)),
        299 => Some((0, 1)),
        300 => Some((0, 2)),
        301 => Some((0, 3)),
        302 => Some((1, 0)),
        303 => Some((1, 1)),
        304 => Some((1, 2)),
        305 => Some((1, 3)),
        306 => Some((2, 0)),
        307 => Some((2, 1)),
        308 => Some((2, 2)),
        309 => Some((2, 3)),
        310 => Some((3, 0)),
        311 => Some((3, 1)),
        312 => Some((3, 2)),
        313 => Some((3, 3)),
        314 => Some((4, 0)),
        315 => Some((4, 1)),
        316 => Some((4, 2)),
        317 => Some((4, 3)),
        318 => Some((6, 0)),
        319 => Some((7, 5)),
        320 => Some((8, 5)),
        321 => Some((10, 1)),
        322 => Some((11, 0)),
        323 => Some((10, 2)),
        324 => Some((11, 2)),
        325 => Some((10, 4)),
        326 => Some((11, 4)),
        327 => Some((12, 4)),
        328 => Some((7, 8)),
        329 => Some((8, 6)),
        330 => Some((12, 2)),
        331 => Some((8, 3)),
        332 => Some((14, 0)),
        333 => Some((8, 8)),
        334 => Some((7, 6)),
        335 => Some((13, 4)),
        336 => Some((6, 1)),
        337 => Some((9, 3)),
        338 => Some((11, 1)),
        339 => Some((10, 3)),
        340 => Some((11, 3)),
        341 => Some((14, 1)),
        342 => Some((7, 9)),
        343 => Some((7, 10)),
        344 => Some((12, 0)),
        345 => Some((6, 3)),
        346 => Some((5, 4)),
        347 => Some((6, 4)),
        348 => Some((9, 4)),
        349 => Some((9, 5)),
        350 => Some((10, 5)),
        351 => Some((14, 4)),
        352 => Some((12, 1)),
        353 => Some((13, 0)),
        354 => Some((13, 1)),
        355 => Some((13, 2)),
        356 => Some((6, 5)),
        357 => Some((12, 5)),
        358 => Some((12, 3)),
        359 => Some((13, 5)),
        360 => Some((13, 6)),
        361 => Some((13, 3)),
        362 => Some((14, 3)),
        363 => Some((9, 6)),
        364 => Some((10, 6)),
        365 => Some((9, 7)),
        366 => Some((10, 7)),
        367 => Some((11, 5)),
        368 => Some((11, 6)),
        369 => Some((12, 6)),
        370 => Some((11, 7)),
        371 => Some((12, 7)),
        372 => Some((13, 7)),
        373 => Some((12, 8)),
        374 => Some((12, 8)),
        375 => Some((11, 8)),
        376 => Some((10, 8)),
        377 => Some((13, 9)),
        378 => Some((13, 10)),
        379 => Some((12, 10)),
        380 => Some((12, 9)),
        381 => Some((11, 9)),
        382 => Some((9, 8)),
        383 => Some((9, 9)),
        384 => Some((11, 10)),
        385 => Some((14, 2)),
        388 => Some((10, 11)),
        389 => Some((14, 12)),
        390 => Some((13, 11)),
        391 => Some((8, 7)),
        392 => Some((7, 7)),
        393 => Some((6, 7)),
        394 => Some((6, 8)),
        395 => Some((12, 13)),
        396 => Some((6, 9)),
        397 => Some((0, 14)),
        398 => Some((6, 6)),
        399 => Some((11, 9)),
        400 => Some((8, 9)),
        401 => Some((12, 9)),
        402 => Some((12, 10)),
        403 => Some((15, 12)),
        404 => Some((9, 5)),
        405 => Some((5, 10)),
        406 => Some((12, 12)),
        407 => Some((12, 7)),
        408 => Some((11, 7)),
        417 => Some((9, 2)),
        418 => Some((9, 4)),
        419 => Some((9, 3)),
        420 => Some((10, 4)),
        421 => Some((10, 3)),
        _ => None,
    }
}

fn items_icon_tile_for_item_with_aux(item_id: u16, aux: u16) -> Option<(u8, u8)> {
    if item_id == 373 {
        if (aux & 0x4000) != 0 {
            return Some((10, 9));
        }
        return Some((12, 8));
    }

    if item_id == 397 {
        return Some(match aux.min(4) {
            0 => (0, 14),
            1 => (1, 14),
            2 => (2, 14),
            3 => (3, 14),
            4 => (4, 14),
            _ => (0, 14),
        });
    }

    if item_id == 351 {
        return Some(match aux.min(15) {
            0 => (14, 4),
            1 => (14, 5),
            2 => (14, 6),
            3 => (14, 7),
            4 => (14, 8),
            5 => (14, 9),
            6 => (14, 10),
            7 => (14, 11),
            8 => (15, 4),
            9 => (15, 5),
            10 => (15, 6),
            11 => (15, 7),
            12 => (15, 8),
            13 => (15, 9),
            14 => (15, 10),
            15 => (15, 11),
            _ => (14, 4),
        });
    }

    items_icon_tile_for_item(item_id)
}

fn icon_spec_for_item(item_id: u16, aux: u16) -> Option<(UiIconAtlas, Rect)> {
    if item_id <= 255 {
        return Some((
            UiIconAtlas::Terrain,
            terrain_icon_rect_for_block_with_aux(item_id, aux),
        ));
    }

    if let Some((tile_x, tile_y)) = items_icon_tile_for_item_with_aux(item_id, aux) {
        let icon_index = u16::from(tile_y) * 16 + u16::from(tile_x);
        return Some((UiIconAtlas::Items, items_icon_rect_for_index(icon_index)));
    }

    if let Some(icon_index) = record_icon_index(item_id) {
        return Some((UiIconAtlas::Items, items_icon_rect_for_index(icon_index)));
    }

    None
}

fn apply_item_icon_to_image(
    image: &mut ImageNode,
    item_id: u16,
    aux: u16,
    atlases: &UiIconAtlasHandles,
) -> bool {
    let Some((atlas_kind, rect)) = icon_spec_for_item(item_id, aux) else {
        return false;
    };

    let texture = match atlas_kind {
        UiIconAtlas::Terrain => atlases.terrain.clone(),
        UiIconAtlas::Items => atlases.items.clone().or_else(|| atlases.terrain.clone()),
    };

    let Some(texture) = texture else {
        return false;
    };

    image.image = texture;
    image.rect = Some(rect);
    true
}

fn block_prefers_aux_icon_overlay(item_id: u16, aux: u16) -> bool {
    if aux == 0 {
        return false;
    }

    matches!(
        item_id,
        5 | 6 | 17 | 18 | 24 | 31 | 35 | 44 | 97 | 98 | 126 | 139 | 155 | 171
    )
}

fn item_prefers_icon_overlay(item_id: u16, aux: u16) -> bool {
    item_id > 255 || block_prefers_aux_icon_overlay(item_id, aux)
}

fn legacy_font_glyph_rect(character: char) -> Rect {
    let glyph_index = u8::try_from(character as u32).unwrap_or(b'?');
    let glyph_size = 8.0;
    let column = f32::from(glyph_index % 16);
    let row = f32::from(glyph_index / 16);
    Rect::new(
        column * glyph_size,
        row * glyph_size,
        (column + 1.0) * glyph_size,
        (row + 1.0) * glyph_size,
    )
}

fn spawn_legacy_bitmap_label<B: ChildBuild>(
    parent: &mut B,
    text: &str,
    font_texture: &Handle<Image>,
    glyph_scale: f32,
) {
    parent
        .spawn((Node {
            display: Display::Flex,
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|glyph_row| {
            for character in text.chars() {
                if character == ' ' {
                    glyph_row.spawn((Node {
                        width: Val::Px(4.0 * glyph_scale),
                        height: Val::Px(8.0 * glyph_scale),
                        ..default()
                    },));
                    continue;
                }

                glyph_row.spawn((
                    Node {
                        width: Val::Px(8.0 * glyph_scale),
                        height: Val::Px(8.0 * glyph_scale),
                        ..default()
                    },
                    ImageNode::new(font_texture.clone())
                        .with_rect(legacy_font_glyph_rect(character)),
                ));
            }
        });
}

fn sync_legacy_menu_button_visuals(
    mut button_query: Query<
        (&Interaction, &LegacyMenuButtonUi, &Children, &mut ImageNode),
        With<Button>,
    >,
    mut label_query: Query<&mut TextColor, With<LegacyMenuButtonLabelUi>>,
) {
    for (interaction, legacy_button, children, mut image_node) in &mut button_query {
        let y_image = if !legacy_button.active {
            0
        } else if *interaction == Interaction::Hovered {
            2
        } else {
            1
        };

        image_node.rect = Some(legacy_button_rect(y_image));

        let label_color = if !legacy_button.active {
            Color::srgb_u8(0xA0, 0xA0, 0xA0)
        } else if *interaction == Interaction::Hovered {
            Color::srgb_u8(0xFF, 0xFF, 0x55)
        } else {
            Color::srgb_u8(0xE0, 0xE0, 0xE0)
        };

        for child in children.iter() {
            if let Ok(mut text_color) = label_query.get_mut(*child) {
                text_color.0 = label_color;
            }
        }
    }
}

fn spawn_legacy_menu_button<B: ChildBuild, M: Component>(
    parent: &mut B,
    gui_texture: Option<Handle<Image>>,
    mojangles_font: Option<Handle<Font>>,
    label: &str,
    top_percent: f32,
    marker: M,
    active: bool,
) {
    let image_node = if let Some(gui_texture) = gui_texture {
        ImageNode::new(gui_texture).with_rect(legacy_button_rect(if active { 1 } else { 0 }))
    } else {
        ImageNode::default()
    };

    let mut button = parent.spawn((
        Button,
        marker,
        LegacyMenuButtonUi { active },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent((100.0 - LEGACY_MENU_BUTTON_WIDTH_PERCENT) * 0.5),
            top: Val::Percent(top_percent),
            width: Val::Percent(LEGACY_MENU_BUTTON_WIDTH_PERCENT),
            height: Val::Percent(LEGACY_MENU_BUTTON_HEIGHT_PERCENT),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
        image_node,
    ));

    button.with_children(|button| {
        let mut text_font = TextFont {
            font_size: 12.0 * LEGACY_MENU_GUI_SCALE,
            ..default()
        };
        if let Some(mojangles_font) = mojangles_font {
            text_font.font = mojangles_font;
        }

        button.spawn((
            LegacyMenuButtonLabelUi,
            Text::new(label),
            text_font,
            TextColor(if active {
                Color::srgb_u8(0xE0, 0xE0, 0xE0)
            } else {
                Color::srgb_u8(0xA0, 0xA0, 0xA0)
            }),
        ));
    });
}

fn spawn_hotbar_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    let gui_texture = runtime_assets
        .0
        .gui_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let terrain_texture = runtime_assets
        .0
        .terrain_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let items_texture = runtime_assets
        .0
        .items_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let fallback_icon_texture = terrain_texture.clone().or(items_texture.clone());

    let hotbar_width = 182.0 * HOTBAR_GUI_SCALE;
    let hotbar_height = 22.0 * HOTBAR_GUI_SCALE;

    commands
        .spawn((
            HotbarRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(8.0),
                width: Val::Px(hotbar_width),
                height: Val::Px(hotbar_height),
                margin: UiRect::left(Val::Px(-hotbar_width * 0.5)),
                ..default()
            },
        ))
        .with_children(|root| {
            if let Some(gui_texture) = gui_texture.clone() {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(hotbar_width),
                        height: Val::Px(hotbar_height),
                        ..default()
                    },
                    ImageNode::new(gui_texture).with_rect(Rect::new(0.0, 0.0, 182.0, 22.0)),
                ));
            } else {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(hotbar_width),
                        height: Val::Px(hotbar_height),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.65)),
                    BorderColor(Color::srgba(0.8, 0.8, 0.8, 0.5)),
                ));
            }

            if let Some(gui_texture) = gui_texture.clone() {
                root.spawn((
                    HotbarSelectionUi,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(-HOTBAR_GUI_SCALE),
                        top: Val::Px(-HOTBAR_GUI_SCALE),
                        width: Val::Px(24.0 * HOTBAR_GUI_SCALE),
                        height: Val::Px(22.0 * HOTBAR_GUI_SCALE),
                        ..default()
                    },
                    ImageNode::new(gui_texture).with_rect(Rect::new(0.0, 22.0, 24.0, 44.0)),
                ));
            }

            for slot in 0..HOTBAR_SLOTS {
                let slot_left = (3.0 + (slot as f32 * 20.0)) * HOTBAR_GUI_SCALE;
                let slot_top = 3.0 * HOTBAR_GUI_SCALE;

                let icon_node = if let Some(texture) = fallback_icon_texture.as_ref() {
                    ImageNode::new(texture.clone()).with_rect(terrain_icon_rect_for_block(1))
                } else {
                    ImageNode::default()
                };

                let mut slot_node = root.spawn((
                    HotbarSlotUi { slot },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(slot_left),
                        top: Val::Px(slot_top),
                        width: Val::Px(16.0 * HOTBAR_GUI_SCALE),
                        height: Val::Px(16.0 * HOTBAR_GUI_SCALE),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.14)),
                    BorderColor(Color::NONE),
                ));

                slot_node.with_children(|slot_parent| {
                    slot_parent.spawn((
                        HotbarItemIconUi { slot },
                        Node {
                            width: Val::Px(16.0 * HOTBAR_GUI_SCALE),
                            height: Val::Px(16.0 * HOTBAR_GUI_SCALE),
                            ..default()
                        },
                        icon_node,
                        Visibility::Hidden,
                    ));
                });
            }
        });
}

fn spawn_health_hud(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    let Some(icons_texture_path) = runtime_assets.0.icons_texture_asset_path.as_ref() else {
        return;
    };

    let icons_texture = asset_server.load(icons_texture_path.clone());
    let row_width = HUD_STATUS_ROW_WIDTH * HOTBAR_GUI_SCALE;
    let hotbar_width = 182.0 * HOTBAR_GUI_SCALE;

    commands
        .spawn((
            HealthHudRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-hotbar_width * 0.5)),
                bottom: Val::Px(status_row_bottom_offset()),
                width: Val::Px(row_width),
                height: Val::Px(HEART_ICON_SIZE * HOTBAR_GUI_SCALE),
                ..default()
            },
        ))
        .with_children(|root| {
            for index in 0..10 {
                let left = (index as f32 * HEART_ICON_STRIDE) * HOTBAR_GUI_SCALE;
                let size = HEART_ICON_SIZE * HOTBAR_GUI_SCALE;

                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(left),
                        width: Val::Px(size),
                        height: Val::Px(size),
                        ..default()
                    },
                    ImageNode::new(icons_texture.clone())
                        .with_rect(Rect::new(16.0, 0.0, 25.0, 9.0)),
                ));

                root.spawn((
                    HeartFillUi { index },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(left),
                        width: Val::Px(size),
                        height: Val::Px(size),
                        ..default()
                    },
                    ImageNode::new(icons_texture.clone())
                        .with_rect(Rect::new(52.0, 0.0, 61.0, 9.0)),
                ));
            }
        });
}

fn spawn_hunger_hud(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    let Some(icons_texture_path) = runtime_assets.0.icons_texture_asset_path.as_ref() else {
        return;
    };

    let icons_texture = asset_server.load(icons_texture_path.clone());
    let hotbar_width = HUD_XP_BAR_WIDTH * HOTBAR_GUI_SCALE;

    commands
        .spawn((
            HungerHudRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-hotbar_width * 0.5)),
                bottom: Val::Px(status_row_bottom_offset()),
                width: Val::Px(hotbar_width),
                height: Val::Px(HEART_ICON_SIZE * HOTBAR_GUI_SCALE),
                ..default()
            },
        ))
        .with_children(|root| {
            for index in 0..10 {
                let left = ((173.0 - index as f32 * HEART_ICON_STRIDE) * HOTBAR_GUI_SCALE).max(0.0);
                let size = HEART_ICON_SIZE * HOTBAR_GUI_SCALE;

                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(left),
                        width: Val::Px(size),
                        height: Val::Px(size),
                        ..default()
                    },
                    ImageNode::new(icons_texture.clone()).with_rect(Rect::new(
                        16.0,
                        HUD_FOOD_UV_Y,
                        25.0,
                        HUD_FOOD_UV_Y + HEART_ICON_SIZE,
                    )),
                ));

                root.spawn((
                    HungerFillUi { index },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(left),
                        width: Val::Px(size),
                        height: Val::Px(size),
                        ..default()
                    },
                    ImageNode::new(icons_texture.clone()).with_rect(Rect::new(
                        52.0,
                        HUD_FOOD_UV_Y,
                        61.0,
                        HUD_FOOD_UV_Y + HEART_ICON_SIZE,
                    )),
                ));
            }
        });
}

fn spawn_xp_hud(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    let Some(icons_texture_path) = runtime_assets.0.icons_texture_asset_path.as_ref() else {
        return;
    };

    let icons_texture = asset_server.load(icons_texture_path.clone());
    let bar_width = HUD_XP_BAR_WIDTH * HOTBAR_GUI_SCALE;
    let xp_bar_height = HUD_XP_BAR_HEIGHT * HOTBAR_GUI_SCALE;

    commands
        .spawn((
            XpHudRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-bar_width * 0.5)),
                bottom: Val::Px(xp_bar_bottom_offset()),
                width: Val::Px(bar_width),
                height: Val::Px(xp_bar_height),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    width: Val::Px(bar_width),
                    height: Val::Px(xp_bar_height),
                    ..default()
                },
                ImageNode::new(icons_texture.clone()).with_rect(Rect::new(
                    0.0,
                    HUD_XP_UV_BG_Y,
                    HUD_XP_BAR_WIDTH,
                    HUD_XP_UV_BG_Y + HUD_XP_BAR_HEIGHT,
                )),
            ));

            root.spawn((
                XpHudFillUi,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    width: Val::Px(0.0),
                    height: Val::Px(xp_bar_height),
                    ..default()
                },
                ImageNode::new(icons_texture).with_rect(Rect::new(
                    0.0,
                    HUD_XP_UV_FILL_Y,
                    HUD_XP_BAR_WIDTH,
                    HUD_XP_UV_FILL_Y + HUD_XP_BAR_HEIGHT,
                )),
            ));
        });

    commands.spawn((
        XpLevelTextUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-12.0 * HOTBAR_GUI_SCALE)),
            bottom: Val::Px(hotbar_top_offset() + 18.0 * HOTBAR_GUI_SCALE),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: 9.0 * HOTBAR_GUI_SCALE,
            ..default()
        },
        TextColor(Color::srgb_u8(0x80, 0xFF, 0x20)),
    ));
}

fn spawn_inventory_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    let terrain_texture = runtime_assets
        .0
        .terrain_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let items_texture = runtime_assets
        .0
        .items_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let fallback_icon_texture = terrain_texture.clone().or(items_texture.clone());
    let panel_width = 176.0 * INVENTORY_GUI_SCALE;
    let panel_height = 166.0 * INVENTORY_GUI_SCALE;

    commands
        .spawn((
            InventoryScreenRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                display: Display::None,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.50)),
        ))
        .with_children(|root| {
            root.spawn((Node {
                width: Val::Px(panel_width),
                height: Val::Px(panel_height),
                position_type: PositionType::Relative,
                ..default()
            },))
                .with_children(|panel| {
                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(panel_width),
                            height: Val::Px(panel_height),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(0xB7, 0xB7, 0xB7)),
                        BorderColor(Color::srgb_u8(0x1A, 0x1A, 0x1A)),
                    ));

                    panel.spawn((Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(8.0 * INVENTORY_GUI_SCALE),
                        top: Val::Px(8.0 * INVENTORY_GUI_SCALE),
                        width: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                        height: Val::Px(18.0 * INVENTORY_GUI_SCALE * 4.0),
                        ..default()
                    },));

                    for armor_slot in 0..4 {
                        panel.spawn((
                            InventoryArmorSlotUi,
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(8.0 * INVENTORY_GUI_SCALE),
                                top: Val::Px(
                                    (8.0 + armor_slot as f32 * 18.0) * INVENTORY_GUI_SCALE,
                                ),
                                width: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                                height: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb_u8(0xD4, 0xD4, 0xD4)),
                            BorderColor(Color::srgb_u8(0x8A, 0x8A, 0x8A)),
                        ));
                    }

                    for slot in 0..INVENTORY_SLOTS {
                        let (slot_x, slot_y) = inventory_slot_origin(slot);
                        let icon_node = if let Some(texture) = fallback_icon_texture.as_ref() {
                            ImageNode::new(texture.clone())
                                .with_rect(terrain_icon_rect_for_block(1))
                        } else {
                            ImageNode::default()
                        };

                        let mut slot_node = panel.spawn((
                            Button,
                            InventorySlotUi { slot },
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(slot_x * INVENTORY_GUI_SCALE),
                                top: Val::Px(slot_y * INVENTORY_GUI_SCALE),
                                width: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                                height: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb_u8(0x92, 0x92, 0x92)),
                            BorderColor(Color::srgb_u8(0xEA, 0xEA, 0xEA)),
                        ));

                        slot_node.with_children(|slot_parent| {
                            slot_parent.spawn((
                                InventoryItemIconUi { slot },
                                Node {
                                    width: Val::Px(16.0 * INVENTORY_GUI_SCALE),
                                    height: Val::Px(16.0 * INVENTORY_GUI_SCALE),
                                    ..default()
                                },
                                icon_node,
                                Visibility::Hidden,
                            ));
                        });
                    }

                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(8.0 * INVENTORY_GUI_SCALE),
                            top: Val::Px(72.0 * INVENTORY_GUI_SCALE),
                            ..default()
                        },
                        Text::new("Inventory"),
                        TextFont {
                            font_size: 10.0 * INVENTORY_GUI_SCALE,
                            ..default()
                        },
                        TextColor(Color::srgb_u8(0x40, 0x40, 0x40)),
                    ));
                });
        });
}

fn spawn_creative_inventory_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    let panel_width = CREATIVE_PANEL_WIDTH * INVENTORY_GUI_SCALE;
    let panel_height = CREATIVE_PANEL_HEIGHT * INVENTORY_GUI_SCALE;
    let icon_size = 16.0 * INVENTORY_GUI_SCALE;
    let tab_width = (CREATIVE_PANEL_WIDTH / CREATIVE_TABS.len() as f32) * INVENTORY_GUI_SCALE;
    let tab_height = 18.0 * INVENTORY_GUI_SCALE;
    let terrain_texture = runtime_assets
        .0
        .terrain_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let items_texture = runtime_assets
        .0
        .items_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let fallback_icon_texture = terrain_texture.clone().or(items_texture.clone());

    commands
        .spawn((
            CreativeInventoryRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                display: Display::None,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.50)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(panel_width),
                    height: Val::Px(panel_height),
                    position_type: PositionType::Relative,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(0xB7, 0xB7, 0xB7)),
                BorderColor(Color::srgb_u8(0x1A, 0x1A, 0x1A)),
            ))
            .with_children(|panel| {
                panel
                    .spawn((Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(8.0 * INVENTORY_GUI_SCALE),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },))
                    .with_children(|row| {
                        row.spawn((
                            CreativeTabLabelUi,
                            Text::new("Building Blocks"),
                            TextFont {
                                font_size: 9.0 * INVENTORY_GUI_SCALE,
                                ..default()
                            },
                            TextColor(Color::srgb_u8(0x40, 0x40, 0x40)),
                        ));
                    });

                panel.spawn((
                    CreativePageLabelUi,
                    Node {
                        position_type: PositionType::Absolute,
                        display: Display::None,
                        ..default()
                    },
                    Text::new(""),
                    TextFont {
                        font_size: 7.0 * INVENTORY_GUI_SCALE,
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0x40, 0x40, 0x40)),
                ));

                for (tab_index, tab) in CREATIVE_TABS.iter().enumerate() {
                    let tab_icon_item_id = creative_tab_icon_item_id(*tab);
                    panel
                        .spawn((
                            Button,
                            CreativeTabButtonUi { tab: *tab },
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(tab_index as f32 * tab_width),
                                top: Val::Px(-18.0 * INVENTORY_GUI_SCALE),
                                width: Val::Px(tab_width),
                                height: Val::Px(tab_height),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb_u8(0xB2, 0xB2, 0xB2)),
                            BorderColor(Color::srgb_u8(0x1A, 0x1A, 0x1A)),
                        ))
                        .with_children(|button| {
                            let icon_node = if let Some((atlas_kind, rect)) =
                                icon_spec_for_item(tab_icon_item_id, 0)
                            {
                                let texture = match atlas_kind {
                                    UiIconAtlas::Terrain => terrain_texture.clone(),
                                    UiIconAtlas::Items => {
                                        items_texture.clone().or_else(|| terrain_texture.clone())
                                    }
                                };

                                if let Some(texture) = texture {
                                    ImageNode::new(texture).with_rect(rect)
                                } else {
                                    ImageNode::default()
                                }
                            } else {
                                ImageNode::default()
                            };

                            button.spawn((
                                Node {
                                    width: Val::Px(14.0 * INVENTORY_GUI_SCALE),
                                    height: Val::Px(14.0 * INVENTORY_GUI_SCALE),
                                    ..default()
                                },
                                icon_node,
                            ));
                        });
                }

                panel
                    .spawn((
                        Button,
                        CreativeInventoryPlayerTabButtonUi,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(183.0 * INVENTORY_GUI_SCALE),
                            top: Val::Px(14.0 * INVENTORY_GUI_SCALE),
                            width: Val::Px(10.0 * INVENTORY_GUI_SCALE),
                            height: Val::Px(16.0 * INVENTORY_GUI_SCALE),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(0xB2, 0xB2, 0xB2)),
                        BorderColor(Color::srgb_u8(0x1A, 0x1A, 0x1A)),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("P"),
                            TextFont {
                                font_size: 6.0 * INVENTORY_GUI_SCALE,
                                ..default()
                            },
                            TextColor(Color::srgb_u8(0x38, 0x38, 0x38)),
                        ));
                    });

                panel.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(184.0 * INVENTORY_GUI_SCALE),
                        top: Val::Px(34.0 * INVENTORY_GUI_SCALE),
                        width: Val::Px(8.0 * INVENTORY_GUI_SCALE),
                        height: Val::Px(90.0 * INVENTORY_GUI_SCALE),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(0xE5, 0xE5, 0xE5)),
                    BorderColor(Color::srgb_u8(0x7A, 0x7A, 0x7A)),
                ));

                panel.spawn((
                    CreativeScrollbarThumbUi,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(183.0 * INVENTORY_GUI_SCALE),
                        top: Val::Px(34.0 * INVENTORY_GUI_SCALE),
                        width: Val::Px(10.0 * INVENTORY_GUI_SCALE),
                        height: Val::Px(10.0 * INVENTORY_GUI_SCALE),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(0xD8, 0xD8, 0xD8)),
                    BorderColor(Color::srgb_u8(0x1A, 0x1A, 0x1A)),
                ));

                panel.spawn((
                    CreativeScrollbarArrowUi,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(183.0 * INVENTORY_GUI_SCALE),
                        top: Val::Px(126.0 * INVENTORY_GUI_SCALE),
                        ..default()
                    },
                    Text::new("▼"),
                    TextFont {
                        font_size: 8.0 * INVENTORY_GUI_SCALE,
                        ..default()
                    },
                    TextColor(Color::srgb_u8(0xF8, 0xD0, 0x42)),
                ));

                for row in 0..CREATIVE_SELECTOR_ROWS {
                    for column in 0..CREATIVE_SELECTOR_COLUMNS {
                        let slot = row * CREATIVE_SELECTOR_COLUMNS + column;

                        let mut slot_button = panel.spawn((
                            Button,
                            CreativeSelectorSlotButtonUi { slot },
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px((8.0 + (column as f32 * 18.0)) * INVENTORY_GUI_SCALE),
                                top: Val::Px((34.0 + (row as f32 * 18.0)) * INVENTORY_GUI_SCALE),
                                width: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                                height: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb_u8(0x92, 0x92, 0x92)),
                            BorderColor(Color::srgb_u8(0xEA, 0xEA, 0xEA)),
                        ));

                        let icon_node = if let Some(texture) = fallback_icon_texture.as_ref() {
                            ImageNode::new(texture.clone())
                                .with_rect(terrain_icon_rect_for_block(1))
                        } else {
                            ImageNode::default()
                        };

                        slot_button.with_children(|button| {
                            button.spawn((
                                CreativeSelectorItemIconUi { slot },
                                Node {
                                    width: Val::Px(icon_size),
                                    height: Val::Px(icon_size),
                                    ..default()
                                },
                                icon_node,
                                Visibility::Hidden,
                            ));
                        });
                    }
                }

                for slot in 0..HOTBAR_SLOTS {
                    let mut hotbar_slot = panel.spawn((
                        Button,
                        CreativeHotbarSlotUi { slot },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px((17.0 + (slot as f32 * 18.0)) * INVENTORY_GUI_SCALE),
                            top: Val::Px(142.0 * INVENTORY_GUI_SCALE),
                            width: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                            height: Val::Px(18.0 * INVENTORY_GUI_SCALE),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(0x92, 0x92, 0x92)),
                        BorderColor(Color::srgb_u8(0xEA, 0xEA, 0xEA)),
                    ));

                    let icon_node = if let Some(texture) = fallback_icon_texture.as_ref() {
                        ImageNode::new(texture.clone()).with_rect(terrain_icon_rect_for_block(1))
                    } else {
                        ImageNode::default()
                    };

                    hotbar_slot.with_children(|slot_parent| {
                        slot_parent.spawn((
                            CreativeHotbarItemIconUi { slot },
                            Node {
                                width: Val::Px(icon_size),
                                height: Val::Px(icon_size),
                                ..default()
                            },
                            icon_node,
                            Visibility::Hidden,
                        ));
                    });
                }
            });
        });
}

fn spawn_pause_menu_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    let gui_texture = runtime_assets
        .0
        .gui_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let menu_logo_texture = runtime_assets
        .0
        .menu_logo_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let mojangles_font = runtime_assets
        .0
        .mojangles_font_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));

    commands
        .spawn((
            PauseMenuRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.50)),
        ))
        .with_children(|root| {
            if let Some(menu_logo_texture) = menu_logo_texture {
                root.spawn((
                    PauseMenuLogoUi,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(LEGACY_MENU_LOGO_FIRST_LEFT_PERCENT),
                        top: Val::Percent(LEGACY_MENU_LOGO_TOP_PERCENT),
                        width: Val::Percent(LEGACY_MENU_LOGO_PART_WIDTH_PERCENT),
                        height: Val::Percent(LEGACY_MENU_LOGO_PART_HEIGHT_PERCENT),
                        ..default()
                    },
                    ImageNode::new(menu_logo_texture.clone())
                        .with_rect(Rect::new(0.0, 0.0, 155.0, 44.0)),
                ));

                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(LEGACY_MENU_LOGO_SECOND_LEFT_PERCENT),
                        top: Val::Percent(LEGACY_MENU_LOGO_TOP_PERCENT),
                        width: Val::Percent(LEGACY_MENU_LOGO_PART_WIDTH_PERCENT),
                        height: Val::Percent(LEGACY_MENU_LOGO_PART_HEIGHT_PERCENT),
                        ..default()
                    },
                    ImageNode::new(menu_logo_texture).with_rect(Rect::new(0.0, 45.0, 155.0, 89.0)),
                ));
            }

            let button_step = LEGACY_MENU_BUTTON_STEP_PERCENT;
            spawn_legacy_menu_button(
                root,
                gui_texture.clone(),
                mojangles_font.clone(),
                "Resume Game",
                LEGACY_MENU_BUTTONS_TOP_PERCENT,
                PauseResumeButtonUi,
                true,
            );
            spawn_legacy_menu_button(
                root,
                gui_texture.clone(),
                mojangles_font.clone(),
                "Help & Options",
                LEGACY_MENU_BUTTONS_TOP_PERCENT + button_step,
                PauseHelpOptionsButtonUi,
                true,
            );
            spawn_legacy_menu_button(
                root,
                gui_texture.clone(),
                mojangles_font.clone(),
                "Leaderboards",
                LEGACY_MENU_BUTTONS_TOP_PERCENT + button_step * 2.0,
                PauseLeaderboardsButtonUi,
                true,
            );
            spawn_legacy_menu_button(
                root,
                gui_texture.clone(),
                mojangles_font.clone(),
                "Achievements",
                LEGACY_MENU_BUTTONS_TOP_PERCENT + button_step * 3.0,
                PauseAchievementsButtonUi,
                true,
            );
            spawn_legacy_menu_button(
                root,
                gui_texture.clone(),
                mojangles_font.clone(),
                "Save Game",
                LEGACY_MENU_BUTTONS_TOP_PERCENT + button_step * 4.0,
                PauseSaveQuitButtonUi,
                true,
            );
            spawn_legacy_menu_button(
                root,
                gui_texture,
                mojangles_font,
                "Exit Game",
                LEGACY_MENU_BUTTONS_TOP_PERCENT + button_step * 5.0,
                PauseExitButtonUi,
                true,
            );
        });
}

fn spawn_death_screen_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) {
    let gui_texture = runtime_assets
        .0
        .gui_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let legacy_font_texture = runtime_assets
        .0
        .font_texture_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));
    let mojangles_font = runtime_assets
        .0
        .mojangles_font_asset_path
        .as_ref()
        .map(|path| asset_server.load(path.clone()));

    commands
        .spawn((
            DeathScreenRootUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.38, 0.13, 0.13, 0.82)),
        ))
        .with_children(|root| {
            root.spawn((Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                ..default()
            },))
                .with_children(|title_row| {
                    if let Some(mojangles_font) = mojangles_font.as_ref() {
                        title_row.spawn((
                            Text::new("Game over!"),
                            TextFont {
                                font: mojangles_font.clone(),
                                font_size: 32.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    } else if let Some(font_texture) = legacy_font_texture.as_ref() {
                        spawn_legacy_bitmap_label(title_row, "Game over!", font_texture, 4.0);
                    } else {
                        title_row.spawn((
                            Text::new("Game over!"),
                            TextFont {
                                font_size: 32.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    }
                });

            root.spawn((Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },))
                .with_children(|score_row| {
                    let mut score_font = TextFont {
                        font_size: 14.0,
                        ..default()
                    };
                    if let Some(mojangles_font) = mojangles_font.as_ref() {
                        score_font.font = mojangles_font.clone();
                    }

                    score_row.spawn((
                        DeathScoreTextUi,
                        Text::new("Score: --"),
                        score_font,
                        TextColor(Color::WHITE),
                    ));
                });

            root.spawn((Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(25.0),
                width: Val::Px(0.0),
                height: Val::Px(0.0),
                ..default()
            },))
                .with_children(|anchor| {
                    let mut respawn_button = anchor.spawn((
                        Button,
                        DeathRespawnButtonUi,
                        LegacyMenuButtonUi { active: true },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(-100.0),
                            top: Val::Px(72.0),
                            width: Val::Px(200.0),
                            height: Val::Px(20.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
                    ));

                    if let Some(gui_texture) = gui_texture.clone() {
                        respawn_button
                            .insert(ImageNode::new(gui_texture).with_rect(legacy_button_rect(1)));
                    }

                    respawn_button.with_children(|button| {
                        if let Some(mojangles_font) = mojangles_font.as_ref() {
                            button.spawn((
                                Text::new("Respawn"),
                                TextFont {
                                    font: mojangles_font.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb_u8(0xE0, 0xE0, 0xE0)),
                            ));
                        } else if let Some(font_texture) = legacy_font_texture.as_ref() {
                            spawn_legacy_bitmap_label(button, "Respawn", font_texture, 1.5);
                        } else {
                            button.spawn((
                                Text::new("Respawn"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb_u8(0xE0, 0xE0, 0xE0)),
                            ));
                        }
                    });

                    let mut title_button = anchor.spawn((
                        Button,
                        DeathQuitButtonUi,
                        LegacyMenuButtonUi { active: true },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(-100.0),
                            top: Val::Px(96.0),
                            width: Val::Px(200.0),
                            height: Val::Px(20.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
                    ));

                    if let Some(gui_texture) = gui_texture {
                        title_button
                            .insert(ImageNode::new(gui_texture).with_rect(legacy_button_rect(1)));
                    }

                    title_button.with_children(|button| {
                        if let Some(mojangles_font) = mojangles_font.as_ref() {
                            button.spawn((
                                Text::new("Title menu"),
                                TextFont {
                                    font: mojangles_font.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb_u8(0xE0, 0xE0, 0xE0)),
                            ));
                        } else if let Some(font_texture) = legacy_font_texture.as_ref() {
                            spawn_legacy_bitmap_label(button, "Title menu", font_texture, 1.5);
                        } else {
                            button.spawn((
                                Text::new("Title menu"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb_u8(0xE0, 0xE0, 0xE0)),
                            ));
                        }
                    });
                });
        });
}

fn inventory_slot_origin(slot: usize) -> (f32, f32) {
    if slot < HOTBAR_SLOTS {
        return (8.0 + (slot as f32 * 18.0), 142.0);
    }

    let inventory_slot = slot - HOTBAR_SLOTS;
    let row = (inventory_slot / 9) as f32;
    let column = (inventory_slot % 9) as f32;

    (8.0 + (column * 18.0), 84.0 + (row * 18.0))
}

fn inventory_crafting_recipe_origin(recipe_index: usize) -> (f32, f32) {
    let row = (recipe_index / 2) as f32;
    let column = (recipe_index % 2) as f32;
    (88.0 + (column * 18.0), 26.0 + (row * 18.0))
}

fn inventory_crafting_recipe_center(recipe_index: usize) -> Vec2 {
    let (slot_x, slot_y) = inventory_crafting_recipe_origin(recipe_index);
    let panel_width = 176.0 * INVENTORY_GUI_SCALE;
    let panel_height = 166.0 * INVENTORY_GUI_SCALE;
    let slot_size = 16.0 * INVENTORY_GUI_SCALE;

    Vec2::new(
        -panel_width * 0.5 + (slot_x * INVENTORY_GUI_SCALE) + slot_size * 0.5,
        panel_height * 0.5 - (slot_y * INVENTORY_GUI_SCALE) - slot_size * 0.5,
    )
}

fn sync_gameplay_overlay_visibility(
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    mut gameplay_ui_query: Query<
        &mut Node,
        Or<(
            With<HotbarRootUi>,
            With<CrosshairRootUi>,
            With<HealthHudRootUi>,
            With<HungerHudRootUi>,
            With<XpHudRootUi>,
            With<XpLevelTextUi>,
        )>,
    >,
) {
    let hide_gameplay_ui = hide_gameplay_overlay(
        inventory_ui.open,
        pause_menu.open,
        game.session.player().is_dead,
    );

    for mut node in &mut gameplay_ui_query {
        node.display = if hide_gameplay_ui {
            Display::None
        } else {
            Display::Flex
        };
    }
}

fn sync_hotbar_ui(
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    ui_icon_atlases: Res<UiIconAtlasHandles>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut mesh_cache: ResMut<UiItemMeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut slot_query: Query<(&HotbarSlotUi, &mut BackgroundColor, &mut BorderColor)>,
    mut selection_query: Query<&mut Node, With<HotbarSelectionUi>>,
    mut model_query: Query<
        (
            &HotbarItemModelUi,
            &mut Transform,
            &mut Visibility,
            &mut Mesh3d,
        ),
        Without<HotbarItemIconUi>,
    >,
    mut icon_query: Query<
        (&HotbarItemIconUi, &mut Visibility, &mut ImageNode),
        Without<HotbarItemModelUi>,
    >,
) {
    let hide_overlay = hide_gameplay_overlay(
        inventory_ui.open,
        pause_menu.open,
        game.session.player().is_dead,
    );

    let states = collect_hotbar_state(&game.session.player().inventory);
    let selected_slot = states
        .iter()
        .find(|state| state.selected)
        .map(|state| state.slot)
        .unwrap_or(0);

    for mut selection_node in &mut selection_query {
        selection_node.left = Val::Px((-1.0 + (selected_slot as f32 * 20.0)) * HOTBAR_GUI_SCALE);
        selection_node.top = Val::Px(-HOTBAR_GUI_SCALE);
    }

    for (slot_ui, mut background, mut border) in &mut slot_query {
        let state = states[slot_ui.slot];
        let has_item = state.item_id.is_some();

        if has_item {
            background.0 = Color::srgba(0.0, 0.0, 0.0, 0.10);
            border.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
        } else {
            background.0 = Color::srgba(0.0, 0.0, 0.0, 0.06);
            border.0 = Color::NONE;
        }
    }

    let hotbar_slot_center_y = match window_query.get_single() {
        Ok(window) => hotbar_slot_center_y(window.height()),
        Err(_) => {
            for (_, _, mut visibility, _) in &mut model_query {
                *visibility = Visibility::Hidden;
            }
            for (_, mut visibility, _) in &mut icon_query {
                *visibility = Visibility::Hidden;
            }
            return;
        }
    };

    for (model_ui, mut transform, mut visibility, mut mesh) in &mut model_query {
        let state = states[model_ui.slot];

        if hide_overlay {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some(item_id) = state.item_id else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let aux = state.aux.unwrap_or(0);

        if item_prefers_icon_overlay(item_id, aux) {
            *visibility = Visibility::Hidden;
            continue;
        }

        mesh.0 = ui_item_mesh_handle(item_id, &mut mesh_cache, &mut meshes);
        *transform = Transform {
            translation: Vec3::new(
                hotbar_slot_center_x(model_ui.slot),
                hotbar_slot_center_y,
                0.0,
            ),
            rotation: ui_item_rotation(),
            scale: ui_item_gui_scale(16.0 * HOTBAR_GUI_SCALE * UI_ITEM_MODEL_SCALE),
        };
        *visibility = Visibility::Visible;
    }

    for (icon_ui, mut visibility, mut image) in &mut icon_query {
        let state = states[icon_ui.slot];

        if hide_overlay {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some(item_id) = state.item_id else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let aux = state.aux.unwrap_or(0);

        if !item_prefers_icon_overlay(item_id, aux) {
            *visibility = Visibility::Hidden;
            continue;
        }

        if apply_item_icon_to_image(&mut image, item_id, aux, &ui_icon_atlases) {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn sync_first_person_item_in_hand(
    fixed_time: Res<Time<Fixed>>,
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    cursor_capture: Res<CursorCaptureState>,
    look_bob_state: Res<LookBobState>,
    ui_icon_atlases: Res<UiIconAtlasHandles>,
    ui_item_assets: Res<UiItemRenderAssets>,
    item_in_hand_state: Res<ItemInHandAnimationState>,
    mut mesh_cache: ResMut<UiItemMeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut first_person_query: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            Has<FirstPersonHeldItemUi>,
            Has<FirstPersonHandUi>,
        ),
        (
            Or<(With<FirstPersonHeldItemUi>, With<FirstPersonHandUi>)>,
            Without<FirstPersonHeldItemIconUi>,
        ),
    >,
    mut first_person_icon_query: Query<
        (&mut Node, &mut ImageNode, &mut Visibility),
        (
            With<FirstPersonHeldItemIconUi>,
            Without<FirstPersonHeldItemUi>,
            Without<FirstPersonHandUi>,
        ),
    >,
) {
    let is_gameplay_view = allow_first_person_item_view(
        cursor_capture.captured,
        inventory_ui.open,
        pause_menu.open,
        game.session.player().is_dead,
    );
    let partial_tick = fixed_time.overstep_fraction().clamp(0.0, 1.0);
    let attack_anim = item_in_hand_state.attack_anim(partial_tick);
    let equip_height = item_in_hand_state.equip_height(partial_tick);
    let use_animation = item_in_hand_state.use_animation();
    let use_ticks = item_in_hand_state.use_ticks(partial_tick);
    let xr = look_bob_state.pitch_old_degrees
        + (look_bob_state.pitch_degrees - look_bob_state.pitch_old_degrees) * partial_tick;
    let yr = look_bob_state.yaw_old_degrees
        + (look_bob_state.yaw_degrees - look_bob_state.yaw_old_degrees) * partial_tick;
    let xrr = look_bob_state.x_bob_old_degrees
        + (look_bob_state.x_bob_degrees - look_bob_state.x_bob_old_degrees) * partial_tick;
    let yrr = look_bob_state.y_bob_old_degrees
        + (look_bob_state.y_bob_degrees - look_bob_state.y_bob_old_degrees) * partial_tick;
    let wobble_pitch_degrees = (xr - xrr) * 0.1;
    let wobble_yaw_degrees = (yr - yrr) * 0.1;

    let selected_stack = game.session.player().inventory.selected_stack();
    let selected_item_id = selected_stack.map(|stack| stack.item_id);
    let selected_is_map = selected_item_id == Some(MAP_ITEM_ID);
    let selected_aux = selected_stack.map(|stack| stack.aux).unwrap_or(0);
    let selected_prefers_icon = selected_item_id
        .map(|item_id| item_prefers_icon_overlay(item_id, selected_aux))
        .unwrap_or(false);
    let selected_block_id = if selected_prefers_icon {
        None
    } else {
        selected_item_id.filter(|item_id| *item_id <= 255)
    };
    let selected_icon_mesh = selected_item_id
        .filter(|_| selected_block_id.is_none())
        .and_then(|item_id| {
            ui_item_icon_mesh_handle(item_id, selected_aux, &mut mesh_cache, &mut meshes)
        });
    let selected_icon_material = selected_item_id
        .filter(|_| selected_block_id.is_none())
        .and_then(|item_id| icon_spec_for_item(item_id, selected_aux))
        .and_then(|(atlas, _)| match atlas {
            UiIconAtlas::Terrain => Some(ui_item_assets.icon_material.clone()),
            UiIconAtlas::Items => ui_item_assets
                .items_material
                .clone()
                .or_else(|| Some(ui_item_assets.icon_material.clone())),
        });
    let render_icon_as_3d_mesh = selected_icon_mesh.is_some() && selected_icon_material.is_some();

    for (mut transform, mut visibility, mut mesh, mut material, is_held_ui, is_hand_ui) in
        &mut first_person_query
    {
        if is_held_ui {
            if !is_gameplay_view {
                *visibility = Visibility::Hidden;
                continue;
            }

            if let Some(block_id) = selected_block_id {
                mesh.0 = ui_item_mesh_handle(block_id, &mut mesh_cache, &mut meshes);
                material.0 = ui_item_assets.material.clone();
                let mut rendered = if selected_is_map {
                    first_person_map_transform(attack_anim, equip_height, xr)
                } else {
                    first_person_held_item_transform(
                        attack_anim,
                        equip_height,
                        use_animation,
                        use_ticks,
                    )
                };
                apply_first_person_wobble(&mut rendered, wobble_pitch_degrees, wobble_yaw_degrees);
                *transform = rendered;
                *visibility = Visibility::Visible;
                continue;
            }

            if render_icon_as_3d_mesh {
                mesh.0 = selected_icon_mesh.clone().expect("mesh should exist");
                material.0 = selected_icon_material
                    .clone()
                    .expect("material should exist");
                let mut rendered = if selected_is_map {
                    first_person_map_transform(attack_anim, equip_height, xr)
                } else {
                    first_person_held_icon_item_transform(
                        attack_anim,
                        equip_height,
                        use_animation,
                        use_ticks,
                    )
                };
                apply_first_person_wobble(&mut rendered, wobble_pitch_degrees, wobble_yaw_degrees);
                *transform = rendered;
                *visibility = Visibility::Visible;
                continue;
            }

            *visibility = Visibility::Hidden;
            continue;
        }

        if is_hand_ui {
            if !is_gameplay_view || selected_item_id.is_some() {
                *visibility = Visibility::Hidden;
                continue;
            }

            let mut rendered = first_person_hand_transform(attack_anim, equip_height);
            apply_first_person_wobble(&mut rendered, wobble_pitch_degrees, wobble_yaw_degrees);
            *transform = rendered;
            *visibility = Visibility::Visible;
        }
    }

    for (mut icon_node, mut icon_image, mut icon_visibility) in &mut first_person_icon_query {
        if !is_gameplay_view {
            *icon_visibility = Visibility::Hidden;
            continue;
        }

        let Some(item_id) = selected_item_id else {
            *icon_visibility = Visibility::Hidden;
            continue;
        };

        if selected_block_id.is_some() {
            *icon_visibility = Visibility::Hidden;
            continue;
        }

        if render_icon_as_3d_mesh {
            *icon_visibility = Visibility::Hidden;
            continue;
        }

        if !apply_item_icon_to_image(&mut icon_image, item_id, selected_aux, &ui_icon_atlases) {
            *icon_visibility = Visibility::Hidden;
            continue;
        }

        let swing = attack_anim
            .clamp(0.0, 1.0)
            .powf(ITEM_IN_HAND_SWING_POW_FACTOR);
        let swing_sqrt = swing.sqrt();
        let swing1 = (swing * std::f32::consts::PI).sin();
        let swing2 = (swing_sqrt * std::f32::consts::PI).sin();
        let equip_offset = (1.0 - equip_height).clamp(0.0, 1.0);

        icon_node.right = Val::Px(120.0 + swing2 * 16.0);
        icon_node.bottom = Val::Px(72.0 - equip_offset * 22.0 + swing1 * 6.0);
        *icon_visibility = Visibility::Visible;
    }
}

fn sync_health_ui(
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    pause_menu: Res<PauseMenuState>,
    mut ui_queries: ParamSet<(
        Query<&mut Node, (With<HealthHudRootUi>, Without<HeartFillUi>)>,
        Query<&mut Node, (With<HungerHudRootUi>, Without<HungerFillUi>)>,
        Query<&mut Node, (With<XpHudRootUi>, Without<XpHudFillUi>)>,
        Query<(&mut Node, &mut ImageNode), (With<XpHudFillUi>, Without<XpHudRootUi>)>,
        Query<(&mut Node, &mut Text), With<XpLevelTextUi>>,
        Query<(&HeartFillUi, &mut Node, &mut ImageNode), Without<HealthHudRootUi>>,
        Query<(&HungerFillUi, &mut Node, &mut ImageNode), Without<HungerHudRootUi>>,
    )>,
) {
    let show_health = !game.session.player().allow_flight
        && !hide_gameplay_overlay(
            inventory_ui.open,
            pause_menu.open,
            game.session.player().is_dead,
        );

    {
        let mut query = ui_queries.p0();
        for mut root_node in &mut query {
            root_node.display = if show_health {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    {
        let mut query = ui_queries.p1();
        for mut root_node in &mut query {
            root_node.display = if show_health {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    {
        let mut query = ui_queries.p2();
        for mut root_node in &mut query {
            root_node.display = if show_health {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    {
        let mut query = ui_queries.p4();
        for (mut text_node, mut level_text) in &mut query {
            text_node.display = if show_health {
                Display::Flex
            } else {
                Display::None
            };

            if !show_health {
                level_text.0.clear();
            }
        }
    }

    if !show_health {
        {
            let mut query = ui_queries.p5();
            for (_, mut node, _) in &mut query {
                node.display = Display::None;
            }
        }

        {
            let mut query = ui_queries.p6();
            for (_, mut node, _) in &mut query {
                node.display = Display::None;
            }
        }

        {
            let mut query = ui_queries.p3();
            for (mut xp_fill_node, _) in &mut query {
                xp_fill_node.display = Display::None;
                xp_fill_node.width = Val::Px(0.0);
            }
        }

        return;
    }

    let health_points = i32::from(game.session.player().health.max(0));

    {
        let mut query = ui_queries.p5();
        for (heart_fill, mut node, mut image_node) in &mut query {
            let remaining = health_points - (heart_fill.index as i32 * 2);

            if remaining >= 2 {
                node.display = Display::Flex;
                image_node.rect = Some(Rect::new(52.0, 0.0, 61.0, 9.0));
            } else if remaining == 1 {
                node.display = Display::Flex;
                image_node.rect = Some(Rect::new(61.0, 0.0, 70.0, 9.0));
            } else {
                node.display = Display::None;
            }
        }
    }

    let food_points = i32::from(game.session.player().food_level.clamp(0, 20));
    {
        let mut query = ui_queries.p6();
        for (hunger_fill, mut node, mut image_node) in &mut query {
            let remaining = food_points - (hunger_fill.index as i32 * 2);

            if remaining >= 2 {
                node.display = Display::Flex;
                image_node.rect = Some(Rect::new(52.0, HUD_FOOD_UV_Y, 61.0, HUD_FOOD_UV_Y + 9.0));
            } else if remaining == 1 {
                node.display = Display::Flex;
                image_node.rect = Some(Rect::new(61.0, HUD_FOOD_UV_Y, 70.0, HUD_FOOD_UV_Y + 9.0));
            } else {
                node.display = Display::None;
            }
        }
    }

    let xp_progress = game.session.player().experience_progress.clamp(0.0, 1.0);
    let xp_fill_pixels = ((HUD_XP_BAR_WIDTH + 1.0) * xp_progress).floor();
    {
        let mut query = ui_queries.p3();
        for (mut xp_fill_node, mut xp_fill_image) in &mut query {
            if xp_fill_pixels <= 0.0 {
                xp_fill_node.display = Display::None;
                xp_fill_node.width = Val::Px(0.0);
                continue;
            }

            xp_fill_node.display = Display::Flex;
            xp_fill_node.width = Val::Px(xp_fill_pixels * HOTBAR_GUI_SCALE);
            xp_fill_image.rect = Some(Rect::new(
                0.0,
                HUD_XP_UV_FILL_Y,
                xp_fill_pixels,
                HUD_XP_UV_FILL_Y + HUD_XP_BAR_HEIGHT,
            ));
        }
    }

    let xp_level = game.session.player().experience_level.max(0);
    {
        let mut query = ui_queries.p4();
        for (mut text_node, mut level_text) in &mut query {
            if xp_level > 0 {
                text_node.display = Display::Flex;
                level_text.0 = xp_level.to_string();
            } else {
                text_node.display = Display::None;
                level_text.0.clear();
            }
        }
    }
}

fn sync_inventory_ui(
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    creative_ui: Res<CreativeInventoryState>,
    ui_icon_atlases: Res<UiIconAtlasHandles>,
    mut mesh_cache: ResMut<UiItemMeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut inventory_root_query: Query<
        &mut Node,
        (With<InventoryScreenRootUi>, Without<InventoryItemModelUi>),
    >,
    mut slot_query: Query<
        (&InventorySlotUi, &mut BackgroundColor, &mut BorderColor),
        Without<CraftingRecipeButtonUi>,
    >,
    mut crafting_button_query: Query<
        (
            &CraftingRecipeButtonUi,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<InventorySlotUi>,
    >,
    mut crafting_count_query: Query<
        (&CraftingRecipeCountUi, &mut TextColor),
        Without<CraftingStatusTextUi>,
    >,
    mut crafting_status_query: Query<
        &mut Text,
        (With<CraftingStatusTextUi>, Without<CraftingRecipeCountUi>),
    >,
    mut model_query: Query<
        (
            &InventoryItemModelUi,
            &mut Transform,
            &mut Visibility,
            &mut Mesh3d,
        ),
        (
            Without<InventoryScreenRootUi>,
            Without<CraftingRecipeModelUi>,
            Without<InventoryItemIconUi>,
            Without<CraftingRecipeIconUi>,
        ),
    >,
    mut crafting_model_query: Query<
        (
            &CraftingRecipeModelUi,
            &mut Transform,
            &mut Visibility,
            &mut Mesh3d,
        ),
        (
            Without<InventoryScreenRootUi>,
            Without<InventoryItemModelUi>,
            Without<InventoryItemIconUi>,
            Without<CraftingRecipeIconUi>,
        ),
    >,
    mut icon_query: Query<
        (&InventoryItemIconUi, &mut Visibility, &mut ImageNode),
        (
            Without<InventoryItemModelUi>,
            Without<CraftingRecipeIconUi>,
            Without<CraftingRecipeModelUi>,
        ),
    >,
    mut crafting_icon_query: Query<
        (&CraftingRecipeIconUi, &mut Visibility, &mut ImageNode),
        (
            Without<CraftingRecipeModelUi>,
            Without<InventoryItemIconUi>,
            Without<InventoryItemModelUi>,
        ),
    >,
) {
    let show_survival_inventory = inventory_ui.open
        && (!game.session.player().allow_flight || creative_ui.show_player_inventory_tab);
    let show_crafting_overlay = false;

    for mut root_node in &mut inventory_root_query {
        root_node.display = if show_survival_inventory {
            Display::Flex
        } else {
            Display::None
        };
    }

    let states = collect_inventory_state(&game.session.player().inventory);
    let crafting_states = collect_crafting_recipe_state(&game.session.player().inventory);

    for mut status_text in &mut crafting_status_query {
        if !show_survival_inventory {
            status_text.0.clear();
        } else if status_text.0.is_empty() {
            status_text.0 = "Click recipe to craft".to_string();
        }
    }

    for (slot_ui, mut background, mut border) in &mut slot_query {
        let state = states[slot_ui.slot];

        if state.selected_hotbar_slot {
            background.0 = Color::srgb_u8(0xA8, 0xA8, 0xA8);
            border.0 = Color::srgb_u8(0xFF, 0xFF, 0xFF);
        } else {
            background.0 = Color::srgb_u8(0x92, 0x92, 0x92);
            border.0 = Color::srgb_u8(0xEA, 0xEA, 0xEA);
        }
    }

    for (recipe_button, mut background, mut border) in &mut crafting_button_query {
        if !show_crafting_overlay {
            background.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
            border.0 = Color::NONE;
            continue;
        }

        let craftable = crafting_states
            .iter()
            .find(|state| state.recipe_id == recipe_button.recipe_id)
            .is_some_and(|state| state.craftable);

        if craftable {
            background.0 = Color::srgba(0.16, 0.28, 0.16, 0.72);
            border.0 = Color::srgba(0.56, 0.80, 0.56, 0.85);
        } else {
            background.0 = Color::srgba(0.28, 0.12, 0.12, 0.72);
            border.0 = Color::srgba(0.72, 0.42, 0.42, 0.82);
        }

        if !show_survival_inventory {
            background.0 = Color::srgba(0.0, 0.0, 0.0, 0.10);
            border.0 = Color::NONE;
        }
    }

    for (recipe_count, mut text_color) in &mut crafting_count_query {
        if !show_crafting_overlay {
            text_color.0 = Color::srgb(0.0, 0.0, 0.0);
            continue;
        }

        let craftable = crafting_states
            .iter()
            .find(|state| state.recipe_id == recipe_count.recipe_id)
            .is_some_and(|state| state.craftable);

        text_color.0 = if craftable {
            Color::srgb(0.95, 0.95, 0.95)
        } else {
            Color::srgb(0.70, 0.70, 0.70)
        };
    }

    for (model_ui, mut transform, mut visibility, mut mesh) in &mut model_query {
        if !show_survival_inventory {
            *visibility = Visibility::Hidden;
            continue;
        }

        let state = states[model_ui.slot];
        let Some(item_id) = state.item_id else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let aux = state.aux.unwrap_or(0);

        if item_prefers_icon_overlay(item_id, aux) {
            *visibility = Visibility::Hidden;
            continue;
        }

        let slot_center = inventory_slot_center(model_ui.slot);
        mesh.0 = ui_item_mesh_handle(item_id, &mut mesh_cache, &mut meshes);
        *transform = Transform {
            translation: Vec3::new(slot_center.x, slot_center.y, 0.0),
            rotation: ui_item_rotation(),
            scale: ui_item_gui_scale(16.0 * INVENTORY_GUI_SCALE * UI_ITEM_MODEL_SCALE),
        };
        *visibility = Visibility::Visible;
    }

    for (recipe_model, mut transform, mut visibility, mut mesh) in &mut crafting_model_query {
        if !show_survival_inventory || !show_crafting_overlay {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some(recipe) = recipe_by_id(recipe_model.recipe_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        if item_prefers_icon_overlay(recipe.output_item_id, 0) {
            *visibility = Visibility::Hidden;
            continue;
        }

        let slot_center = inventory_crafting_recipe_center(recipe_model.recipe_index);
        mesh.0 = ui_item_mesh_handle(recipe.output_item_id, &mut mesh_cache, &mut meshes);
        *transform = Transform {
            translation: Vec3::new(slot_center.x, slot_center.y, 0.0),
            rotation: ui_item_rotation(),
            scale: ui_item_gui_scale(16.0 * INVENTORY_GUI_SCALE * UI_ITEM_MODEL_SCALE),
        };
        *visibility = Visibility::Visible;
    }

    for (icon_ui, mut visibility, mut image) in &mut icon_query {
        if !show_survival_inventory {
            *visibility = Visibility::Hidden;
            continue;
        }

        let state = states[icon_ui.slot];
        let Some(item_id) = state.item_id else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let aux = state.aux.unwrap_or(0);

        if !item_prefers_icon_overlay(item_id, aux) {
            *visibility = Visibility::Hidden;
            continue;
        }

        if apply_item_icon_to_image(&mut image, item_id, aux, &ui_icon_atlases) {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    for (icon_ui, mut visibility, mut image) in &mut crafting_icon_query {
        if !show_survival_inventory || !show_crafting_overlay {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some(recipe) = recipe_by_id(icon_ui.recipe_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        if !item_prefers_icon_overlay(recipe.output_item_id, 0) {
            *visibility = Visibility::Hidden;
            continue;
        }

        if apply_item_icon_to_image(&mut image, recipe.output_item_id, 0, &ui_icon_atlases) {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn sync_inventory_player_preview(
    fixed_time: Res<Time<Fixed>>,
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    creative_ui: Res<CreativeInventoryState>,
    player_walk_animation: Res<PlayerWalkAnimationState>,
    item_in_hand_state: Res<ItemInHandAnimationState>,
    ui_item_assets: Res<UiItemRenderAssets>,
    mut mesh_cache: ResMut<UiItemMeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut root_query: Query<
        (&mut Transform, &mut Visibility),
        (
            With<InventoryPlayerPreviewRootUi>,
            Without<InventoryPlayerPreviewPartUi>,
            Without<InventoryPlayerPreviewHeldItemUi>,
        ),
    >,
    mut part_query: Query<
        (&InventoryPlayerPreviewPartUi, &mut Transform),
        (
            Without<InventoryPlayerPreviewRootUi>,
            Without<InventoryPlayerPreviewHeldItemUi>,
        ),
    >,
    mut held_item_query: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        (
            With<InventoryPlayerPreviewHeldItemUi>,
            Without<InventoryPlayerPreviewRootUi>,
            Without<InventoryPlayerPreviewPartUi>,
        ),
    >,
) {
    let show_survival_inventory = inventory_ui.open
        && (!game.session.player().allow_flight || creative_ui.show_player_inventory_tab);

    if !show_survival_inventory {
        for (_, mut visibility) in &mut root_query {
            *visibility = Visibility::Hidden;
        }
        for (_, mut visibility, _, _) in &mut held_item_query {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let Ok(window) = window_query.get_single() else {
        for (_, mut visibility) in &mut root_query {
            *visibility = Visibility::Hidden;
        }
        for (_, mut visibility, _, _) in &mut held_item_query {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    let panel_width = 176.0 * INVENTORY_GUI_SCALE;
    let panel_height = 166.0 * INVENTORY_GUI_SCALE;
    let panel_x = (window.width() - panel_width) * 0.5;
    let panel_y = (window.height() - panel_height) * 0.5;
    let model_screen_x = panel_x + INVENTORY_PLAYER_PREVIEW_SCREEN_X * INVENTORY_GUI_SCALE;
    let model_screen_y = panel_y + INVENTORY_PLAYER_PREVIEW_SCREEN_Y * INVENTORY_GUI_SCALE;

    let cursor = window
        .cursor_position()
        .unwrap_or(Vec2::new(window.width() * 0.5, window.height() * 0.5));
    let xd = model_screen_x - cursor.x;
    let yd = (model_screen_y - INVENTORY_PLAYER_PREVIEW_CURSOR_Y_OFFSET * INVENTORY_GUI_SCALE)
        - cursor.y;
    let divisor = INVENTORY_PLAYER_PREVIEW_MOUSE_DIVISOR * INVENTORY_GUI_SCALE;
    let yaw_base = (xd / divisor).atan();
    let pitch_base = (yd / divisor).atan();
    let root_pitch = -pitch_base * INVENTORY_PLAYER_PREVIEW_ROTATE_SCALE_DEGREES.to_radians();
    let body_yaw = yaw_base * INVENTORY_PLAYER_PREVIEW_ROTATE_SCALE_DEGREES.to_radians();
    let head_yaw = yaw_base * INVENTORY_PLAYER_PREVIEW_HEAD_YAW_SCALE_DEGREES.to_radians();
    let head_pitch = -pitch_base * INVENTORY_PLAYER_PREVIEW_ROTATE_SCALE_DEGREES.to_radians();
    let player = game.session.player();
    let partial_tick = fixed_time.overstep_fraction().clamp(0.0, 1.0);
    let walk_time = player_walk_animation.walk_dist_old
        + (player_walk_animation.walk_dist - player_walk_animation.walk_dist_old) * partial_tick;
    let walk_speed = player_walk_animation.bob_old
        + (player_walk_animation.bob - player_walk_animation.bob_old) * partial_tick;
    let bob_time = player_walk_animation.age_ticks + partial_tick;
    let attack_time = item_in_hand_state.attack_anim(partial_tick).clamp(0.0, 1.0);
    let use_animation = item_in_hand_state.use_animation();
    let use_ticks = item_in_hand_state.use_ticks(partial_tick);
    let eating_pose = use_animation == HeldItemUseAnimation::EatDrink && use_ticks > 0.0;
    let bow_pose = use_animation == HeldItemUseAnimation::Bow && use_ticks > 0.0;
    let blocking_pose = use_animation == HeldItemUseAnimation::Block && use_ticks > 0.0;
    let holding_right_hand = if player.inventory.selected_stack().is_some() {
        1.0
    } else {
        0.0
    };
    let horizontal_speed =
        (player.velocity.x * player.velocity.x + player.velocity.z * player.velocity.z).sqrt();
    let sneaking = player.is_sneaking;
    let riding = player.is_riding;
    let idle = !sneaking && !riding && horizontal_speed <= 0.01;

    let mut body_part_yaw = 0.0_f32;
    let mut body_part_pitch = 0.0_f32;

    let mut head_position = Vec3::new(0.0, 0.0, 0.0);
    let mut body_position = Vec3::new(0.0, 0.0, 0.0);

    let mut right_arm_position = Vec3::new(-5.0, 2.0, 0.0);
    let mut left_arm_position = Vec3::new(5.0, 2.0, 0.0);
    let mut right_leg_position = Vec3::new(-1.9, 12.0, 0.0);
    let mut left_leg_position = Vec3::new(1.9, 12.0, 0.0);

    let mut right_arm_x_rot = (walk_time * 0.6662 + std::f32::consts::PI).cos() * walk_speed;
    let mut left_arm_x_rot = (walk_time * 0.6662).cos() * walk_speed;
    let mut right_arm_y_rot = 0.0_f32;
    let mut left_arm_y_rot = 0.0_f32;
    let mut right_arm_z_rot = 0.0_f32;
    let mut left_arm_z_rot = 0.0_f32;

    let mut right_leg_x_rot = (walk_time * 0.6662).cos() * 1.4 * walk_speed;
    let mut left_leg_x_rot = (walk_time * 0.6662 + std::f32::consts::PI).cos() * 1.4 * walk_speed;
    let mut right_leg_y_rot = 0.0_f32;
    let mut left_leg_y_rot = 0.0_f32;

    if riding {
        right_arm_x_rot += -std::f32::consts::FRAC_PI_2 * 0.4;
        left_arm_x_rot += -std::f32::consts::FRAC_PI_2 * 0.4;
        right_leg_x_rot = -std::f32::consts::FRAC_PI_2 * 0.8;
        left_leg_x_rot = -std::f32::consts::FRAC_PI_2 * 0.8;
        right_leg_y_rot = std::f32::consts::FRAC_PI_2 * 0.2;
        left_leg_y_rot = -std::f32::consts::FRAC_PI_2 * 0.2;
    } else if idle {
        right_leg_x_rot = -std::f32::consts::FRAC_PI_2;
        left_leg_x_rot = -std::f32::consts::FRAC_PI_2;
        right_leg_y_rot = std::f32::consts::FRAC_PI_2 * 0.2;
        left_leg_y_rot = -std::f32::consts::FRAC_PI_2 * 0.2;
    }

    if holding_right_hand > 0.0 {
        right_arm_x_rot = right_arm_x_rot * 0.5 - std::f32::consts::FRAC_PI_2 * 0.2;
    }

    if attack_time > 0.0 {
        body_part_yaw = (attack_time.sqrt() * std::f32::consts::PI * 2.0).sin() * 0.2;
        right_arm_position.z = body_part_yaw.sin() * 5.0;
        right_arm_position.x = -body_part_yaw.cos() * 5.0;
        left_arm_position.z = -body_part_yaw.sin() * 5.0;
        left_arm_position.x = body_part_yaw.cos() * 5.0;

        right_arm_y_rot += body_part_yaw;
        left_arm_y_rot += body_part_yaw;
        left_arm_x_rot += body_part_yaw;

        let mut swing = 1.0 - attack_time;
        swing *= swing;
        swing *= swing;
        swing = 1.0 - swing;

        let aa = (swing * std::f32::consts::PI).sin();
        let bb = (attack_time * std::f32::consts::PI).sin() * -(head_pitch - 0.7) * 0.75;
        right_arm_x_rot -= aa * 1.2 + bb;
        right_arm_y_rot += body_part_yaw * 2.0;
        right_arm_z_rot = (attack_time * std::f32::consts::PI).sin() * -0.4;
    }

    if eating_pose {
        let eat_swing = (use_ticks / EAT_DRINK_USE_DURATION_TICKS).clamp(0.0, 1.0);
        let eat_t = (EAT_DRINK_USE_DURATION_TICKS - use_ticks).max(0.0);
        let mut eat_inverse = 1.0 - eat_swing;
        eat_inverse = eat_inverse.powi(9);
        let eat_swing_smoothed = 1.0 - eat_inverse;
        let eat_chomp = if eat_swing > 0.2 { 1.0 } else { 0.0 };
        right_arm_x_rot = -(eat_t / 4.0 * std::f32::consts::PI).cos().abs() * 0.1 * eat_chomp * 2.0;
        right_arm_y_rot -= eat_swing_smoothed * 0.5;
        right_arm_x_rot -= eat_swing_smoothed * 1.2;
    }

    if sneaking {
        body_part_pitch = 0.5;
        right_arm_x_rot += 0.4;
        left_arm_x_rot += 0.4;
        right_leg_position.z = 4.0;
        left_leg_position.z = 4.0;
        right_arm_position.y = 2.0;
        left_arm_position.y = 2.0;
        right_leg_position.y = 9.0;
        left_leg_position.y = 9.0;
        head_position.y = 1.0;
    } else {
        body_part_pitch = 0.0;
        right_leg_position.z = 0.1;
        left_leg_position.z = 0.1;

        if idle && !riding {
            right_leg_position.y = 22.0;
            left_leg_position.y = 22.0;
            body_position.y = 10.0;
            right_arm_position.y = 12.0;
            left_arm_position.y = 12.0;
            head_position.y = 10.0;
        } else {
            right_leg_position.y = 12.0;
            left_leg_position.y = 12.0;
            body_position.y = 0.0;
            right_arm_position.y = 2.0;
            left_arm_position.y = 2.0;
            head_position.y = 0.0;
        }
    }

    right_arm_z_rot += (bob_time * 0.09).cos() * 0.05 + 0.05;
    left_arm_z_rot -= (bob_time * 0.09).cos() * 0.05 + 0.05;
    right_arm_x_rot += (bob_time * 0.067).sin() * 0.05;
    left_arm_x_rot -= (bob_time * 0.067).sin() * 0.05;

    if bow_pose {
        right_arm_z_rot = 0.0;
        left_arm_z_rot = 0.0;
        right_arm_y_rot = head_yaw - 0.1;
        left_arm_y_rot = head_yaw + 0.5;
        right_arm_x_rot = -std::f32::consts::FRAC_PI_2 + head_pitch;
        left_arm_x_rot = -std::f32::consts::FRAC_PI_2 + head_pitch;
        right_arm_z_rot += (bob_time * 0.09).cos() * 0.05 + 0.05;
        left_arm_z_rot -= (bob_time * 0.09).cos() * 0.05 + 0.05;
        right_arm_x_rot += (bob_time * 0.067).sin() * 0.05;
        left_arm_x_rot -= (bob_time * 0.067).sin() * 0.05;
    }

    let preview_center = inventory_player_preview_center();

    for (mut root_transform, mut visibility) in &mut root_query {
        root_transform.translation = Vec3::new(preview_center.x, preview_center.y, 12.0);
        root_transform.rotation = Quat::from_rotation_y(std::f32::consts::PI + body_yaw)
            * Quat::from_rotation_x(root_pitch);
        root_transform.scale = Vec3::new(
            INVENTORY_PLAYER_PREVIEW_MODEL_SCALE,
            -INVENTORY_PLAYER_PREVIEW_MODEL_SCALE,
            INVENTORY_PLAYER_PREVIEW_MODEL_SCALE,
        );
        *visibility = Visibility::Visible;
    }

    for (part_ui, mut transform) in &mut part_query {
        transform.scale = Vec3::ONE;
        match part_ui.part {
            InventoryPlayerPreviewPart::Head => {
                transform.translation = head_position;
                transform.rotation = model_part_rotation_from_cxx(head_pitch, head_yaw, 0.0);
            }
            InventoryPlayerPreviewPart::Body => {
                transform.translation = body_position;
                transform.rotation =
                    model_part_rotation_from_cxx(body_part_pitch, body_part_yaw, 0.0);
            }
            InventoryPlayerPreviewPart::RightArm => {
                transform.translation = right_arm_position;
                transform.rotation =
                    model_part_rotation_from_cxx(right_arm_x_rot, right_arm_y_rot, right_arm_z_rot);
            }
            InventoryPlayerPreviewPart::LeftArm => {
                transform.translation = left_arm_position;
                transform.rotation =
                    model_part_rotation_from_cxx(left_arm_x_rot, left_arm_y_rot, left_arm_z_rot);
            }
            InventoryPlayerPreviewPart::RightLeg => {
                transform.translation = right_leg_position;
                transform.rotation =
                    model_part_rotation_from_cxx(right_leg_x_rot, right_leg_y_rot, 0.0);
            }
            InventoryPlayerPreviewPart::LeftLeg => {
                transform.translation = left_leg_position;
                transform.rotation =
                    model_part_rotation_from_cxx(left_leg_x_rot, left_leg_y_rot, 0.0);
            }
        }
    }

    let selected_stack = player.inventory.selected_stack();
    for (mut transform, mut visibility, mut mesh, mut material) in &mut held_item_query {
        let Some(selected_stack) = selected_stack else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let selected_item_id = selected_stack.item_id;
        let selected_aux = selected_stack.aux;
        let selected_prefers_icon = item_prefers_icon_overlay(selected_item_id, selected_aux);
        let selected_block_id = if selected_prefers_icon || selected_item_id > 255 {
            None
        } else {
            Some(selected_item_id)
        };

        if let Some(block_id) = selected_block_id {
            mesh.0 = ui_item_mesh_handle(block_id, &mut mesh_cache, &mut meshes);
            material.0 = ui_item_assets.material.clone();
        } else {
            let Some(icon_mesh) = ui_item_icon_mesh_handle(
                selected_item_id,
                selected_aux,
                &mut mesh_cache,
                &mut meshes,
            ) else {
                *visibility = Visibility::Hidden;
                continue;
            };
            let Some(icon_material) = icon_spec_for_item(selected_item_id, selected_aux).and_then(
                |(atlas, _)| match atlas {
                    UiIconAtlas::Terrain => Some(ui_item_assets.icon_material.clone()),
                    UiIconAtlas::Items => ui_item_assets
                        .items_material
                        .clone()
                        .or_else(|| Some(ui_item_assets.icon_material.clone())),
                },
            ) else {
                *visibility = Visibility::Hidden;
                continue;
            };

            mesh.0 = icon_mesh;
            material.0 = icon_material;
        }

        let item_local = third_person_preview_item_transform(
            selected_item_id,
            selected_block_id.is_some(),
            blocking_pose,
        );
        let arm_local = Transform {
            translation: right_arm_position,
            rotation: model_part_rotation_from_cxx(
                right_arm_x_rot,
                right_arm_y_rot,
                right_arm_z_rot,
            ),
            scale: Vec3::ONE,
        };

        let arm_matrix = Mat4::from_scale_rotation_translation(
            arm_local.scale,
            arm_local.rotation,
            arm_local.translation,
        );
        let item_matrix = Mat4::from_scale_rotation_translation(
            item_local.scale,
            item_local.rotation,
            item_local.translation,
        );
        *transform = transform_from_opengl_sequence(arm_matrix * item_matrix);
        *visibility = Visibility::Visible;
    }
}

fn model_part_rotation_from_cxx(x_rot: f32, y_rot: f32, z_rot: f32) -> Quat {
    Quat::from_rotation_z(z_rot) * Quat::from_rotation_y(y_rot) * Quat::from_rotation_x(x_rot)
}

fn third_person_preview_item_transform(
    item_id: u16,
    is_block_item: bool,
    blocking_with_item: bool,
) -> Transform {
    let mut matrix = Mat4::IDENTITY;
    matrix *= Mat4::from_translation(Vec3::new(-1.0, 7.0, 1.0));

    if is_block_item {
        let mut s = 8.0;
        matrix *= Mat4::from_translation(Vec3::new(0.0, 3.0, -5.0));
        s *= 0.75;
        matrix *= Mat4::from_rotation_x(20.0_f32.to_radians());
        matrix *= Mat4::from_rotation_y(45.0_f32.to_radians());
        matrix *= Mat4::from_scale(Vec3::new(-s, -s, s));
    } else if item_id == BOW_ITEM_ID {
        let s = 10.0;
        matrix *= Mat4::from_translation(Vec3::new(0.0, 2.0, 5.0));
        matrix *= Mat4::from_rotation_y((-20.0_f32).to_radians());
        matrix *= Mat4::from_scale(Vec3::new(s, -s, s));
        matrix *= Mat4::from_rotation_x((-100.0_f32).to_radians());
        matrix *= Mat4::from_rotation_y(45.0_f32.to_radians());
    } else if is_hand_equipped_item(item_id) {
        let s = 10.0;

        if is_mirrored_art_item(item_id) {
            matrix *= Mat4::from_rotation_z(180.0_f32.to_radians());
            matrix *= Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0));
        }

        if blocking_with_item {
            matrix *= Mat4::from_translation(Vec3::new(0.05, 0.0, -0.1));
            matrix *= Mat4::from_rotation_y((-50.0_f32).to_radians());
            matrix *= Mat4::from_rotation_x((-10.0_f32).to_radians());
            matrix *= Mat4::from_rotation_z((-60.0_f32).to_radians());
        }

        matrix *= Mat4::from_translation(Vec3::new(0.0, 3.0, 0.0));
        matrix *= Mat4::from_scale(Vec3::new(s, -s, s));
        matrix *= Mat4::from_rotation_x((-100.0_f32).to_radians());
        matrix *= Mat4::from_rotation_y(45.0_f32.to_radians());
    } else {
        let s = 6.0;
        matrix *= Mat4::from_translation(Vec3::new(4.0, 3.0, -3.0));
        matrix *= Mat4::from_scale(Vec3::splat(s));
        matrix *= Mat4::from_rotation_z(60.0_f32.to_radians());
        matrix *= Mat4::from_rotation_x((-90.0_f32).to_radians());
        matrix *= Mat4::from_rotation_z(20.0_f32.to_radians());
    }

    transform_from_opengl_sequence(matrix)
}

fn sync_creative_inventory_ui(
    game: Res<GameState>,
    inventory_ui: Res<InventoryUiState>,
    mut creative_ui: ResMut<CreativeInventoryState>,
    ui_icon_atlases: Res<UiIconAtlasHandles>,
    mut creative_root_query: Query<
        &mut Node,
        (
            With<CreativeInventoryRootUi>,
            Without<CreativeSelectorSlotButtonUi>,
            Without<CreativeScrollbarThumbUi>,
            Without<CreativeScrollbarArrowUi>,
        ),
    >,
    mut tab_label_query: Query<&mut Text, (With<CreativeTabLabelUi>, Without<CreativePageLabelUi>)>,
    mut page_label_query: Query<
        &mut Text,
        (With<CreativePageLabelUi>, Without<CreativeTabLabelUi>),
    >,
    mut tab_button_query: Query<
        (
            &CreativeTabButtonUi,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<Button>,
            Without<CreativeSelectorSlotButtonUi>,
            Without<CreativeHotbarSlotUi>,
        ),
    >,
    mut player_tab_button_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (
            With<Button>,
            With<CreativeInventoryPlayerTabButtonUi>,
            Without<CreativeTabButtonUi>,
            Without<CreativeSelectorSlotButtonUi>,
            Without<CreativeHotbarSlotUi>,
        ),
    >,
    mut selector_slot_query: Query<
        (
            &CreativeSelectorSlotButtonUi,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<Button>,
            Without<CreativeTabButtonUi>,
            Without<CreativeHotbarSlotUi>,
        ),
    >,
    mut hotbar_slot_query: Query<
        (
            &CreativeHotbarSlotUi,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<Button>,
            Without<CreativeTabButtonUi>,
            Without<CreativeSelectorSlotButtonUi>,
        ),
    >,
    mut selector_icon_query: Query<
        (&CreativeSelectorItemIconUi, &mut Visibility, &mut ImageNode),
        Without<CreativeHotbarItemIconUi>,
    >,
    mut hotbar_icon_query: Query<
        (&CreativeHotbarItemIconUi, &mut Visibility, &mut ImageNode),
        Without<CreativeSelectorItemIconUi>,
    >,
    mut scrollbar_thumb_query: Query<
        &mut Node,
        (
            With<CreativeScrollbarThumbUi>,
            Without<CreativeScrollbarArrowUi>,
        ),
    >,
    mut scrollbar_arrow_query: Query<
        &mut Node,
        (
            With<CreativeScrollbarArrowUi>,
            Without<CreativeScrollbarThumbUi>,
        ),
    >,
) {
    let show_creative_inventory = inventory_ui.open
        && game.session.player().allow_flight
        && !creative_ui.show_player_inventory_tab;

    for mut root_node in &mut creative_root_query {
        root_node.display = if show_creative_inventory {
            Display::Flex
        } else {
            Display::None
        };
    }

    if !show_creative_inventory {
        for (_, mut visibility, _) in &mut selector_icon_query {
            *visibility = Visibility::Hidden;
        }

        for (_, mut visibility, _) in &mut hotbar_icon_query {
            *visibility = Visibility::Hidden;
        }

        for mut arrow_node in &mut scrollbar_arrow_query {
            arrow_node.display = Display::None;
        }

        return;
    }

    let dynamic_group = creative_ui.active_dynamic_group();
    let page_count =
        creative_tab_entry_page_count_for_dynamic_group(creative_ui.tab, dynamic_group);
    let clamped_page = creative_ui.active_page().min(page_count.saturating_sub(1));
    creative_ui.set_active_page(clamped_page);
    let selector_items = creative_selector_entries_page_for_dynamic_group(
        creative_ui.tab,
        dynamic_group,
        clamped_page,
    );

    for mut label in &mut tab_label_query {
        label.0 = creative_tab_title(creative_ui.tab).to_string();
    }

    for mut label in &mut page_label_query {
        label.0.clear();
    }

    for (tab_button, interaction, mut background, mut border) in &mut tab_button_query {
        let active = tab_button.tab == creative_ui.tab;
        let hovered = *interaction == Interaction::Hovered;

        if active {
            background.0 = Color::srgb_u8(0xD5, 0xD5, 0xD5);
            border.0 = Color::srgb_u8(0x1A, 0x1A, 0x1A);
        } else if hovered {
            background.0 = Color::srgb_u8(0xC3, 0xC3, 0xC3);
            border.0 = Color::srgb_u8(0x1A, 0x1A, 0x1A);
        } else {
            background.0 = Color::srgb_u8(0xB2, 0xB2, 0xB2);
            border.0 = Color::srgb_u8(0x1A, 0x1A, 0x1A);
        }
    }

    for (interaction, mut background, mut border) in &mut player_tab_button_query {
        let hovered = *interaction == Interaction::Hovered;
        if hovered {
            background.0 = Color::srgb_u8(0xC3, 0xC3, 0xC3);
            border.0 = Color::srgb_u8(0x1A, 0x1A, 0x1A);
        } else {
            background.0 = Color::srgb_u8(0xB2, 0xB2, 0xB2);
            border.0 = Color::srgb_u8(0x1A, 0x1A, 0x1A);
        }
    }

    for (slot_button, interaction, mut background, mut border) in &mut selector_slot_query {
        let has_item = selector_items[slot_button.slot].is_some();
        let hovered = *interaction == Interaction::Hovered;

        if !has_item {
            background.0 = Color::srgb_u8(0x92, 0x92, 0x92);
            border.0 = Color::srgb_u8(0xEA, 0xEA, 0xEA);
            continue;
        }

        if hovered {
            background.0 = Color::srgb_u8(0xBF, 0xBF, 0xBF);
            border.0 = Color::srgb_u8(0xFF, 0xFF, 0xFF);
        } else {
            background.0 = Color::srgb_u8(0x92, 0x92, 0x92);
            border.0 = Color::srgb_u8(0xEA, 0xEA, 0xEA);
        }
    }

    let hotbar_states = collect_hotbar_state(&game.session.player().inventory);
    for (slot_ui, interaction, mut background, mut border) in &mut hotbar_slot_query {
        let slot_state = hotbar_states[slot_ui.slot];
        let hovered = *interaction == Interaction::Hovered;

        if slot_state.selected {
            background.0 = Color::srgb_u8(0xA8, 0xA8, 0xA8);
            border.0 = Color::srgb_u8(0xFF, 0xFF, 0xFF);
        } else if hovered && slot_state.item_id.is_some() {
            background.0 = Color::srgb_u8(0xBF, 0xBF, 0xBF);
            border.0 = Color::srgb_u8(0xFF, 0xFF, 0xFF);
        } else if slot_state.item_id.is_some() {
            background.0 = Color::srgb_u8(0x92, 0x92, 0x92);
            border.0 = Color::srgb_u8(0xEA, 0xEA, 0xEA);
        } else {
            background.0 = Color::srgb_u8(0x92, 0x92, 0x92);
            border.0 = Color::srgb_u8(0xEA, 0xEA, 0xEA);
        }
    }

    let page_count_for_scroll = page_count.max(1);
    let scroll_progress = if page_count_for_scroll > 1 {
        clamped_page as f32 / (page_count_for_scroll - 1) as f32
    } else {
        0.0
    };
    let thumb_top = (34.0 + scroll_progress * 80.0) * INVENTORY_GUI_SCALE;

    for mut thumb_node in &mut scrollbar_thumb_query {
        thumb_node.top = Val::Px(thumb_top);
    }

    for mut arrow_node in &mut scrollbar_arrow_query {
        arrow_node.display = if page_count_for_scroll > 1 {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (selector_icon, mut visibility, mut image) in &mut selector_icon_query {
        let Some(entry) = selector_items[selector_icon.slot] else {
            *visibility = Visibility::Hidden;
            continue;
        };

        if apply_item_icon_to_image(&mut image, entry.item_id, entry.aux, &ui_icon_atlases) {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    for (hotbar_icon, mut visibility, mut image) in &mut hotbar_icon_query {
        let slot_state = hotbar_states[hotbar_icon.slot];
        let Some(item_id) = slot_state.item_id else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let aux = slot_state.aux.unwrap_or(0);

        if apply_item_icon_to_image(&mut image, item_id, aux, &ui_icon_atlases) {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn sync_chat_ui(
    chat_input: Res<ChatInputState>,
    mut chat_root_query: Query<&mut Node, (With<ChatRootUi>, Without<ChatInputTextUi>)>,
    mut chat_text_query: Query<&mut Text, (With<ChatInputTextUi>, Without<ChatRootUi>)>,
) {
    for mut root_node in &mut chat_root_query {
        root_node.display = if chat_input.open {
            Display::Flex
        } else {
            Display::None
        };
    }

    let display_text = if chat_input.open {
        format!("> {}_", chat_input.text)
    } else {
        String::new()
    };

    for mut text in &mut chat_text_query {
        text.0 = display_text.clone();
    }
}

fn sync_fps_ui(
    diagnostics: Res<DiagnosticsStore>,
    mut fps_query: Query<&mut Text, With<FpsTextUi>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|metric| metric.smoothed());

    for mut text in &mut fps_query {
        if let Some(fps) = fps {
            text.0 = format!("FPS: {:.0}", fps);
        } else {
            text.0 = "FPS: --".to_string();
        }
    }
}

fn hotbar_slot_center_x(slot: usize) -> f32 {
    let hotbar_width = 182.0 * HOTBAR_GUI_SCALE;
    let slot_left = (3.0 + (slot as f32 * 20.0)) * HOTBAR_GUI_SCALE;
    let slot_size = 16.0 * HOTBAR_GUI_SCALE;

    -hotbar_width * 0.5 + slot_left + slot_size * 0.5
}

fn hotbar_slot_center_y(window_height: f32) -> f32 {
    let slot_top = 3.0 * HOTBAR_GUI_SCALE;
    let slot_size = 16.0 * HOTBAR_GUI_SCALE;

    -window_height * 0.5 + HOTBAR_BOTTOM_OFFSET + slot_top + slot_size * 0.5
}

fn inventory_slot_center(slot: usize) -> Vec2 {
    let (slot_x, slot_y) = inventory_slot_origin(slot);
    let panel_width = 176.0 * INVENTORY_GUI_SCALE;
    let panel_height = 166.0 * INVENTORY_GUI_SCALE;
    let slot_size = 18.0 * INVENTORY_GUI_SCALE;

    Vec2::new(
        -panel_width * 0.5 + slot_x * INVENTORY_GUI_SCALE + slot_size * 0.5,
        panel_height * 0.5 - slot_y * INVENTORY_GUI_SCALE - slot_size * 0.5,
    )
}

fn inventory_player_preview_center() -> Vec2 {
    let panel_width = 176.0 * INVENTORY_GUI_SCALE;
    let panel_height = 166.0 * INVENTORY_GUI_SCALE;

    Vec2::new(
        -panel_width * 0.5 + INVENTORY_PLAYER_PREVIEW_SCREEN_X * INVENTORY_GUI_SCALE,
        panel_height * 0.5 - INVENTORY_PLAYER_PREVIEW_SCREEN_Y * INVENTORY_GUI_SCALE,
    )
}

fn ui_item_mesh_handle(
    block_id: u16,
    cache: &mut UiItemMeshCache,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    if let Some(existing) = cache.by_block_id.get(&block_id) {
        return existing.clone();
    }

    let mesh = meshes.add(build_ui_item_mesh(block_id));
    cache.by_block_id.insert(block_id, mesh.clone());
    mesh
}

fn ui_item_icon_mesh_handle(
    item_id: u16,
    aux: u16,
    cache: &mut UiItemMeshCache,
    meshes: &mut Assets<Mesh>,
) -> Option<Handle<Mesh>> {
    let key = (item_id, aux);
    if let Some(existing) = cache.by_item_icon_key.get(&key) {
        return Some(existing.clone());
    }

    let mesh = meshes.add(build_ui_item_icon_mesh(item_id, aux)?);
    cache.by_item_icon_key.insert(key, mesh.clone());
    Some(mesh)
}

fn build_ui_item_icon_mesh(item_id: u16, aux: u16) -> Option<Mesh> {
    let (_, rect) = icon_spec_for_item(item_id, aux)?;
    let atlas_size = 256.0_f32;
    let icon_pixel_width = (rect.max.x - rect.min.x).max(1.0);
    let icon_pixel_height = (rect.max.y - rect.min.y).max(1.0);

    let voxel_count = ITEM_ICON_MESH_PIXELS * ITEM_ICON_MESH_PIXELS;
    let mut positions = Vec::with_capacity(voxel_count * 24);
    let mut normals = Vec::with_capacity(voxel_count * 24);
    let mut uvs = Vec::with_capacity(voxel_count * 24);
    let mut indices = Vec::with_capacity(voxel_count * 36);
    let dd = ITEM_ICON_MESH_DEPTH;

    for y_pixel in 0..ITEM_ICON_MESH_PIXELS {
        for x_pixel in 0..ITEM_ICON_MESH_PIXELS {
            let icon_x = ((ITEM_ICON_MESH_PIXELS - 1 - x_pixel) as f32
                / ITEM_ICON_MESH_PIXELS as f32)
                * icon_pixel_width
                + 0.5;
            let icon_y = ((ITEM_ICON_MESH_PIXELS - 1 - y_pixel) as f32
                / ITEM_ICON_MESH_PIXELS as f32)
                * icon_pixel_height
                + 0.5;
            let icon_u = (rect.min.x + icon_x) / atlas_size;
            let icon_v = (rect.min.y + icon_y) / atlas_size;

            let x0 = x_pixel as f32 / ITEM_ICON_MESH_PIXELS as f32;
            let x1 = x0 + (1.0 / ITEM_ICON_MESH_PIXELS as f32);
            let y0 = y_pixel as f32 / ITEM_ICON_MESH_PIXELS as f32;
            let y1 = y0 + (1.0 / ITEM_ICON_MESH_PIXELS as f32);
            let z0 = 0.0;
            let z1 = -dd;

            append_ui_icon_voxel_face(
                &mut positions,
                &mut normals,
                &mut uvs,
                &mut indices,
                [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
                [0.0, 0.0, 1.0],
                icon_u,
                icon_v,
            );
            append_ui_icon_voxel_face(
                &mut positions,
                &mut normals,
                &mut uvs,
                &mut indices,
                [[x0, y1, z1], [x1, y1, z1], [x1, y0, z1], [x0, y0, z1]],
                [0.0, 0.0, -1.0],
                icon_u,
                icon_v,
            );
            append_ui_icon_voxel_face(
                &mut positions,
                &mut normals,
                &mut uvs,
                &mut indices,
                [[x0, y0, z1], [x0, y0, z0], [x0, y1, z0], [x0, y1, z1]],
                [-1.0, 0.0, 0.0],
                icon_u,
                icon_v,
            );
            append_ui_icon_voxel_face(
                &mut positions,
                &mut normals,
                &mut uvs,
                &mut indices,
                [[x1, y1, z1], [x1, y1, z0], [x1, y0, z0], [x1, y0, z1]],
                [1.0, 0.0, 0.0],
                icon_u,
                icon_v,
            );
            append_ui_icon_voxel_face(
                &mut positions,
                &mut normals,
                &mut uvs,
                &mut indices,
                [[x1, y0, z0], [x0, y0, z0], [x0, y0, z1], [x1, y0, z1]],
                [0.0, 1.0, 0.0],
                icon_u,
                icon_v,
            );
            append_ui_icon_voxel_face(
                &mut positions,
                &mut normals,
                &mut uvs,
                &mut indices,
                [[x1, y1, z1], [x0, y1, z1], [x0, y1, z0], [x1, y1, z0]],
                [0.0, -1.0, 0.0],
                icon_u,
                icon_v,
            );
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

fn append_ui_icon_voxel_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    u: f32,
    v: f32,
) {
    let base = u32::try_from(positions.len()).unwrap_or(u32::MAX - 4);
    for corner in corners {
        positions.push(corner);
        normals.push(normal);
        uvs.push([u, v]);
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[derive(Clone, Copy)]
struct InventoryPlayerPreviewPartSpec {
    part: InventoryPlayerPreviewPart,
    position: Vec3,
    tex_u: u8,
    tex_v: u8,
    x0: f32,
    y0: f32,
    z0: f32,
    width: u8,
    height: u8,
    depth: u8,
    mirror: bool,
}

const INVENTORY_PLAYER_PREVIEW_PART_SPECS: [InventoryPlayerPreviewPartSpec; 6] = [
    InventoryPlayerPreviewPartSpec {
        part: InventoryPlayerPreviewPart::Head,
        position: Vec3::new(0.0, 0.0, 0.0),
        tex_u: 0,
        tex_v: 0,
        x0: -4.0,
        y0: -8.0,
        z0: -4.0,
        width: 8,
        height: 8,
        depth: 8,
        mirror: false,
    },
    InventoryPlayerPreviewPartSpec {
        part: InventoryPlayerPreviewPart::Body,
        position: Vec3::new(0.0, 0.0, 0.0),
        tex_u: 16,
        tex_v: 16,
        x0: -4.0,
        y0: 0.0,
        z0: -2.0,
        width: 8,
        height: 12,
        depth: 4,
        mirror: false,
    },
    InventoryPlayerPreviewPartSpec {
        part: InventoryPlayerPreviewPart::RightArm,
        position: Vec3::new(-5.0, 2.0, 0.0),
        tex_u: 40,
        tex_v: 16,
        x0: -3.0,
        y0: -2.0,
        z0: -2.0,
        width: 4,
        height: 12,
        depth: 4,
        mirror: false,
    },
    InventoryPlayerPreviewPartSpec {
        part: InventoryPlayerPreviewPart::LeftArm,
        position: Vec3::new(5.0, 2.0, 0.0),
        tex_u: 40,
        tex_v: 16,
        x0: -1.0,
        y0: -2.0,
        z0: -2.0,
        width: 4,
        height: 12,
        depth: 4,
        mirror: true,
    },
    InventoryPlayerPreviewPartSpec {
        part: InventoryPlayerPreviewPart::RightLeg,
        position: Vec3::new(-1.9, 12.0, 0.0),
        tex_u: 0,
        tex_v: 16,
        x0: -2.0,
        y0: 0.0,
        z0: -2.0,
        width: 4,
        height: 12,
        depth: 4,
        mirror: false,
    },
    InventoryPlayerPreviewPartSpec {
        part: InventoryPlayerPreviewPart::LeftLeg,
        position: Vec3::new(1.9, 12.0, 0.0),
        tex_u: 0,
        tex_v: 16,
        x0: -2.0,
        y0: 0.0,
        z0: -2.0,
        width: 4,
        height: 12,
        depth: 4,
        mirror: true,
    },
];

fn inventory_player_preview_part_spec(
    part: InventoryPlayerPreviewPart,
) -> InventoryPlayerPreviewPartSpec {
    INVENTORY_PLAYER_PREVIEW_PART_SPECS
        .iter()
        .copied()
        .find(|spec| spec.part == part)
        .expect("all preview parts should have a mesh spec")
}

fn build_inventory_player_preview_part_mesh(spec: InventoryPlayerPreviewPartSpec) -> Mesh {
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    append_humanoid_box_mesh(&mut positions, &mut normals, &mut uvs, &mut indices, spec);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn append_humanoid_box_mesh(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    spec: InventoryPlayerPreviewPartSpec,
) {
    let mut x0 = spec.x0;
    let y0 = spec.y0;
    let z0 = spec.z0;
    let mut x1 = x0 + f32::from(spec.width);
    let y1 = y0 + f32::from(spec.height);
    let z1 = z0 + f32::from(spec.depth);

    if spec.mirror {
        std::mem::swap(&mut x0, &mut x1);
    }

    let u0 = [x0, y0, z0];
    let u1 = [x1, y0, z0];
    let u2 = [x1, y1, z0];
    let u3 = [x0, y1, z0];
    let l0 = [x0, y0, z1];
    let l1 = [x1, y0, z1];
    let l2 = [x1, y1, z1];
    let l3 = [x0, y1, z1];

    let tex_u = f32::from(spec.tex_u);
    let tex_v = f32::from(spec.tex_v);
    let w = f32::from(spec.width);
    let h = f32::from(spec.height);
    let d = f32::from(spec.depth);

    append_humanoid_face(
        positions,
        normals,
        uvs,
        indices,
        [l1, u1, u2, l2],
        tex_u + d + w,
        tex_v + d,
        tex_u + d + w + d,
        tex_v + d + h,
        spec.mirror,
    );
    append_humanoid_face(
        positions,
        normals,
        uvs,
        indices,
        [u0, l0, l3, u3],
        tex_u,
        tex_v + d,
        tex_u + d,
        tex_v + d + h,
        spec.mirror,
    );
    append_humanoid_face(
        positions,
        normals,
        uvs,
        indices,
        [l1, l0, u0, u1],
        tex_u + d,
        tex_v,
        tex_u + d + w,
        tex_v + d,
        spec.mirror,
    );
    append_humanoid_face(
        positions,
        normals,
        uvs,
        indices,
        [u2, u3, l3, l2],
        tex_u + d + w,
        tex_v,
        tex_u + d + w + w,
        tex_v + d,
        spec.mirror,
    );
    append_humanoid_face(
        positions,
        normals,
        uvs,
        indices,
        [u1, u0, u3, u2],
        tex_u + d,
        tex_v + d,
        tex_u + d + w,
        tex_v + d + h,
        spec.mirror,
    );
    append_humanoid_face(
        positions,
        normals,
        uvs,
        indices,
        [l0, l1, l2, l3],
        tex_u + d + w + d,
        tex_v + d,
        tex_u + d + w + d + w,
        tex_v + d + h,
        spec.mirror,
    );
}

fn append_humanoid_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    mut corners: [[f32; 3]; 4],
    u0_px: f32,
    v0_px: f32,
    u1_px: f32,
    v1_px: f32,
    mirror: bool,
) {
    if mirror {
        corners.reverse();
    }

    let base = u32::try_from(positions.len()).unwrap_or(u32::MAX - 4);
    let uv_map = humanoid_face_uvs(u0_px, v0_px, u1_px, v1_px);

    let v0 = Vec3::from_array(corners[0]);
    let v1 = Vec3::from_array(corners[1]);
    let v2 = Vec3::from_array(corners[2]);
    let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero().to_array();

    for (index, corner) in corners.into_iter().enumerate() {
        positions.push(corner);
        normals.push(normal);
        uvs.push(uv_map[index]);
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn humanoid_face_uvs(u0_px: f32, v0_px: f32, u1_px: f32, v1_px: f32) -> [[f32; 2]; 4] {
    let u_scale = 1.0 / 64.0;
    let v_scale = 1.0 / 32.0;
    let u_epsilon = if u1_px > u0_px {
        0.1 * u_scale
    } else {
        -0.1 * u_scale
    };
    let v_epsilon = if v1_px > v0_px {
        0.1 * v_scale
    } else {
        -0.1 * v_scale
    };

    [
        [u1_px * u_scale - u_epsilon, v0_px * v_scale + v_epsilon],
        [u0_px * u_scale + u_epsilon, v0_px * v_scale + v_epsilon],
        [u0_px * u_scale + u_epsilon, v1_px * v_scale - v_epsilon],
        [u1_px * u_scale - u_epsilon, v1_px * v_scale - v_epsilon],
    ]
}

fn build_ui_item_mesh(block_id: u16) -> Mesh {
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    for face in UI_ITEM_FACE_DEFS {
        let base = u32::try_from(positions.len()).unwrap_or(u32::MAX - 4);
        let (tile_x, tile_y) = atlas_tile_for_block_face(block_id, face.face);
        let face_uvs = ui_item_atlas_uv(tile_x, tile_y);

        for (index, corner) in face.corners.iter().enumerate() {
            positions.push(*corner);
            normals.push(face.normal);
            uvs.push(face_uvs[face.uv_indices[index]]);
        }

        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn build_first_person_hand_mesh() -> Mesh {
    let spec = inventory_player_preview_part_spec(InventoryPlayerPreviewPart::RightArm);
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    append_humanoid_box_mesh(&mut positions, &mut normals, &mut uvs, &mut indices, spec);

    let model_scale = 1.0 / 16.0;
    for position in &mut positions {
        position[0] = (position[0] + spec.position.x) * model_scale;
        position[1] = (position[1] + spec.position.y) * model_scale;
        position[2] = (position[2] + spec.position.z) * model_scale;
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn build_break_particle_mesh(tile_x: u8, tile_y: u8, subtile_u: u8, subtile_v: u8) -> Mesh {
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    let face_uvs = break_particle_subtile_uv(tile_x, tile_y, subtile_u, subtile_v);

    for face in UI_ITEM_FACE_DEFS {
        let base = u32::try_from(positions.len()).unwrap_or(u32::MAX - 4);

        for (index, corner) in face.corners.iter().enumerate() {
            positions.push(*corner);
            normals.push(face.normal);
            uvs.push(face_uvs[face.uv_indices[index]]);
        }

        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn break_particle_subtile_uv(
    tile_x: u8,
    tile_y: u8,
    subtile_u: u8,
    subtile_v: u8,
) -> [[f32; 2]; 4] {
    const UV_INSET: f32 = 0.001;
    let tile = 1.0 / f32::from(TERRAIN_ATLAS_TILES);
    let subtile = tile / 4.0;
    let base_u = f32::from(tile_x) * tile + f32::from(subtile_u) * subtile;
    let base_v = f32::from(tile_y) * tile + f32::from(subtile_v) * subtile;
    let u0 = base_u + UV_INSET;
    let v0 = base_v + UV_INSET;
    let u1 = base_u + subtile - UV_INSET;
    let v1 = base_v + subtile - UV_INSET;

    [[u0, v1], [u1, v1], [u1, v0], [u0, v0]]
}

fn ui_item_rotation() -> Quat {
    Quat::from_rotation_x(210.0_f32.to_radians()) * Quat::from_rotation_y((-45.0_f32).to_radians())
}

fn ui_item_gui_scale(scale: f32) -> Vec3 {
    Vec3::new(scale, scale, -scale)
}

fn first_person_hand_transform(attack_anim: f32, equip_height: f32) -> Transform {
    let d = 0.8_f32;
    let swing = attack_anim.clamp(0.0, 1.0);
    let swing_sqrt = swing.sqrt();
    let swing1 = (swing * std::f32::consts::PI).sin();
    let swing2 = (swing_sqrt * std::f32::consts::PI).sin();
    let swing3 = ((swing * swing) * std::f32::consts::PI).sin();

    let mut matrix = Mat4::IDENTITY;
    matrix *= Mat4::from_translation(Vec3::new(
        -swing2 * 0.3,
        (swing_sqrt * std::f32::consts::PI * 2.0).sin() * 0.4,
        -swing1 * 0.4,
    ));
    matrix *= Mat4::from_translation(Vec3::new(
        0.8 * d,
        -0.75 * d - (1.0 - equip_height) * 0.6,
        -0.9 * d,
    ));
    matrix *= Mat4::from_rotation_y(45.0_f32.to_radians());
    matrix *= Mat4::from_rotation_y((swing2 * 70.0).to_radians());
    matrix *= Mat4::from_rotation_z((-swing3 * 20.0).to_radians());

    matrix *= Mat4::from_translation(Vec3::new(-1.0, 3.6, 3.5));
    matrix *= Mat4::from_rotation_z(120.0_f32.to_radians());
    matrix *= Mat4::from_rotation_x((180.0_f32 + 20.0_f32).to_radians());
    matrix *= Mat4::from_rotation_y((-90.0_f32 - 45.0_f32).to_radians());
    matrix *= Mat4::from_translation(Vec3::new(5.6, 0.0, 0.0));

    transform_from_opengl_sequence(matrix)
}

fn first_person_map_transform(
    attack_anim: f32,
    equip_height: f32,
    pitch_degrees: f32,
) -> Transform {
    let d = 0.8_f32;
    let swing = attack_anim.clamp(0.0, 1.0);
    let swing_sqrt = swing.sqrt();
    let swing1 = (swing * std::f32::consts::PI).sin();
    let swing2 = (swing_sqrt * std::f32::consts::PI).sin();
    let swing3 = ((swing * swing) * std::f32::consts::PI).sin();

    let mut tilt = 1.0 - pitch_degrees / 45.0 + 0.1;
    tilt = tilt.clamp(0.0, 1.0);
    tilt = -(tilt * std::f32::consts::PI).cos() * 0.5 + 0.5;

    let mut matrix = Mat4::IDENTITY;
    matrix *= Mat4::from_translation(Vec3::new(
        -swing2 * 0.4,
        (swing_sqrt * std::f32::consts::PI * 2.0).sin() * 0.2,
        -swing1 * 0.2,
    ));
    matrix *= Mat4::from_translation(Vec3::new(
        0.0,
        -(1.0 - equip_height) * 1.2 - tilt * 0.5 + 0.04,
        -0.9 * d,
    ));
    matrix *= Mat4::from_rotation_y(90.0_f32.to_radians());
    matrix *= Mat4::from_rotation_z((tilt * -85.0).to_radians());
    matrix *= Mat4::from_rotation_y((-swing3 * 20.0).to_radians());
    matrix *= Mat4::from_rotation_z((-swing2 * 20.0).to_radians());
    matrix *= Mat4::from_rotation_x((-swing2 * 80.0).to_radians());
    matrix *= Mat4::from_scale(Vec3::splat(0.38));
    matrix *= Mat4::from_rotation_y(90.0_f32.to_radians());
    matrix *= Mat4::from_rotation_z(180.0_f32.to_radians());
    matrix *= Mat4::from_translation(Vec3::new(-1.0, -1.0, 0.0));

    transform_from_opengl_sequence(matrix)
}

fn first_person_held_item_transform(
    attack_anim: f32,
    equip_height: f32,
    use_animation: HeldItemUseAnimation,
    use_ticks: f32,
) -> Transform {
    let d = 0.8_f32;
    let swing = attack_anim
        .clamp(0.0, 1.0)
        .powf(ITEM_IN_HAND_SWING_POW_FACTOR);
    let swing_sqrt = swing.sqrt();
    let swing1 = (swing * std::f32::consts::PI).sin();
    let swing2 = (swing_sqrt * std::f32::consts::PI).sin();
    let swing3 = ((swing * swing) * std::f32::consts::PI).sin();

    let mut matrix = Mat4::IDENTITY;

    if use_animation == HeldItemUseAnimation::EatDrink {
        let use_duration = use_animation_max_duration_ticks(use_animation);
        let use_progress = (use_ticks / use_duration).clamp(0.0, 1.0);
        let mut use_inverse = 1.0 - use_progress;
        use_inverse = use_inverse.powi(9);
        let use_swing = 1.0 - use_inverse;
        let remaining_ticks = (use_duration - use_ticks).max(0.0);
        let eat_shake = ((remaining_ticks / 4.0) * std::f32::consts::PI).cos().abs();
        let eat_chomp = if use_progress > 0.2 { 1.0 } else { 0.0 };

        matrix *= Mat4::from_translation(Vec3::new(0.0, eat_shake * 0.1 * eat_chomp, 0.0));
        matrix *= Mat4::from_translation(Vec3::new(use_swing * 0.6, -use_swing * 0.5, 0.0));
        matrix *= Mat4::from_rotation_y((use_swing * 90.0).to_radians());
        matrix *= Mat4::from_rotation_x((use_swing * 10.0).to_radians());
        matrix *= Mat4::from_rotation_z((use_swing * 30.0).to_radians());
    } else {
        matrix *= Mat4::from_translation(Vec3::new(
            -swing2 * 0.4,
            (swing_sqrt * std::f32::consts::PI * 2.0).sin() * 0.2,
            -swing1 * 0.2,
        ));
    }

    matrix *= Mat4::from_translation(Vec3::new(
        0.7 * d,
        -0.65 * d - (1.0 - equip_height) * 0.6,
        -0.9 * d,
    ));
    matrix *= Mat4::from_rotation_y(45.0_f32.to_radians());
    matrix *= Mat4::from_rotation_y((-swing3 * 20.0).to_radians());
    matrix *= Mat4::from_rotation_z((-swing2 * 20.0).to_radians());
    matrix *= Mat4::from_rotation_x((-swing2 * 80.0).to_radians());
    matrix *= Mat4::from_scale(Vec3::splat(0.4));

    match use_animation {
        HeldItemUseAnimation::Block => {
            matrix *= Mat4::from_translation(Vec3::new(-0.5, 0.2, 0.0));
            matrix *= Mat4::from_rotation_y(30.0_f32.to_radians());
            matrix *= Mat4::from_rotation_x((-80.0_f32).to_radians());
            matrix *= Mat4::from_rotation_y(60.0_f32.to_radians());
        }
        HeldItemUseAnimation::Bow => {
            matrix *= Mat4::from_rotation_z((-18.0_f32).to_radians());
            matrix *= Mat4::from_rotation_y((-12.0_f32).to_radians());
            matrix *= Mat4::from_rotation_x((-8.0_f32).to_radians());
            matrix *= Mat4::from_translation(Vec3::new(-0.9, 0.2, 0.0));

            let mut draw = use_ticks / BOW_DRAW_DURATION_TICKS;
            draw = ((draw * draw) + draw * 2.0) / 3.0;
            draw = draw.clamp(0.0, 1.0);

            if draw > 0.1 {
                matrix *= Mat4::from_translation(Vec3::new(
                    0.0,
                    ((use_ticks - 0.1) * 1.3).sin() * 0.01 * (draw - 0.1),
                    0.0,
                ));
            }

            matrix *= Mat4::from_translation(Vec3::new(0.0, 0.0, draw * 0.1));
            matrix *= Mat4::from_rotation_z((-45.0_f32 - 290.0_f32).to_radians());
            matrix *= Mat4::from_rotation_y((-50.0_f32).to_radians());
            matrix *= Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0));
            matrix *= Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + draw * 0.2));
            matrix *= Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0));
            matrix *= Mat4::from_rotation_y(50.0_f32.to_radians());
            matrix *= Mat4::from_rotation_z((45.0_f32 + 290.0_f32).to_radians());
        }
        _ => {}
    }

    transform_from_opengl_sequence(matrix)
}

fn first_person_held_icon_item_transform(
    attack_anim: f32,
    equip_height: f32,
    use_animation: HeldItemUseAnimation,
    use_ticks: f32,
) -> Transform {
    let base =
        first_person_held_item_transform(attack_anim, equip_height, use_animation, use_ticks);
    let mut matrix =
        Mat4::from_scale_rotation_translation(base.scale, base.rotation, base.translation);

    matrix *= Mat4::from_translation(Vec3::new(0.0, -0.3, 0.0));
    matrix *= Mat4::from_scale(Vec3::splat(1.5));
    matrix *= Mat4::from_rotation_y(50.0_f32.to_radians());
    matrix *= Mat4::from_rotation_z((45.0_f32 + 290.0_f32).to_radians());
    matrix *= Mat4::from_translation(Vec3::new(-15.0 / 16.0, -1.0 / 16.0, 0.0));

    transform_from_opengl_sequence(matrix)
}

fn transform_from_opengl_sequence(matrix: Mat4) -> Transform {
    let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
    Transform {
        translation,
        rotation,
        scale,
    }
}

fn apply_first_person_wobble(
    transform: &mut Transform,
    wobble_pitch_degrees: f32,
    wobble_yaw_degrees: f32,
) {
    let wobble = Mat4::from_rotation_x(wobble_pitch_degrees.to_radians())
        * Mat4::from_rotation_y(wobble_yaw_degrees.to_radians());
    let item = Mat4::from_scale_rotation_translation(
        transform.scale,
        transform.rotation,
        transform.translation,
    );
    let combined = wobble * item;
    let (scale, rotation, translation) = combined.to_scale_rotation_translation();
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = scale;
}

fn ui_item_atlas_uv(tile_x: u8, tile_y: u8) -> [[f32; 2]; 4] {
    const UV_INSET: f32 = 0.001;
    let tile = 1.0 / f32::from(TERRAIN_ATLAS_TILES);
    let u0 = f32::from(tile_x) * tile + UV_INSET;
    let v0 = f32::from(tile_y) * tile + UV_INSET;
    let u1 = (f32::from(tile_x) + 1.0) * tile - UV_INSET;
    let v1 = (f32::from(tile_y) + 1.0) * tile - UV_INSET;

    [[u0, v1], [u1, v1], [u1, v0], [u0, v0]]
}

#[derive(Clone, Copy)]
struct UiItemFaceDef {
    face: BlockFace,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    uv_indices: [usize; 4],
}

const UI_ITEM_FACE_DEFS: [UiItemFaceDef; 6] = [
    UiItemFaceDef {
        face: BlockFace::Top,
        normal: [0.0, 1.0, 0.0],
        corners: [
            [-0.5, 0.5, -0.5],
            [0.5, 0.5, -0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
        ],
        uv_indices: [0, 1, 2, 3],
    },
    UiItemFaceDef {
        face: BlockFace::Bottom,
        normal: [0.0, -1.0, 0.0],
        corners: [
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, -0.5, -0.5],
            [-0.5, -0.5, -0.5],
        ],
        uv_indices: [0, 1, 2, 3],
    },
    UiItemFaceDef {
        face: BlockFace::North,
        normal: [0.0, 0.0, -1.0],
        corners: [
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, -0.5, -0.5],
        ],
        uv_indices: [0, 3, 2, 1],
    },
    UiItemFaceDef {
        face: BlockFace::South,
        normal: [0.0, 0.0, 1.0],
        corners: [
            [-0.5, -0.5, 0.5],
            [-0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [0.5, -0.5, 0.5],
        ],
        uv_indices: [0, 3, 2, 1],
    },
    UiItemFaceDef {
        face: BlockFace::West,
        normal: [-1.0, 0.0, 0.0],
        corners: [
            [-0.5, -0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, 0.5, 0.5],
            [-0.5, -0.5, 0.5],
        ],
        uv_indices: [0, 3, 2, 1],
    },
    UiItemFaceDef {
        face: BlockFace::East,
        normal: [1.0, 0.0, 0.0],
        corners: [
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [0.5, 0.5, -0.5],
            [0.5, -0.5, -0.5],
        ],
        uv_indices: [0, 3, 2, 1],
    },
];

fn build_cloud_mesh(u_offset: f32, v_offset: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let min_section = -CLOUD_ADVANCED_SECTION_RADIUS + 1;
    let max_section = CLOUD_ADVANCED_SECTION_RADIUS;
    for section_x in min_section..=max_section {
        for section_z in min_section..=max_section {
            for texel_local_x in 0..CLOUD_ADVANCED_TEXELS_PER_SECTION {
                for texel_local_z in 0..CLOUD_ADVANCED_TEXELS_PER_SECTION {
                    let texel_x = section_x * CLOUD_ADVANCED_TEXELS_PER_SECTION + texel_local_x;
                    let texel_z = section_z * CLOUD_ADVANCED_TEXELS_PER_SECTION + texel_local_z;

                    let x0 = texel_x as f32 * CLOUD_ADVANCED_TEXEL_WORLD_SIZE;
                    let x1 = x0 + CLOUD_ADVANCED_TEXEL_WORLD_SIZE;
                    let z0 = texel_z as f32 * CLOUD_ADVANCED_TEXEL_WORLD_SIZE;
                    let z1 = z0 + CLOUD_ADVANCED_TEXEL_WORLD_SIZE;
                    let y0 = 0.0;
                    let y1 = CLOUD_LAYER_THICKNESS;

                    push_cloud_quad(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut indices,
                        [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                        [0.0, 1.0, 0.0],
                        texel_x,
                        texel_z,
                        u_offset,
                        v_offset,
                    );
                    push_cloud_quad(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut indices,
                        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                        [0.0, -1.0, 0.0],
                        texel_x,
                        texel_z,
                        u_offset,
                        v_offset,
                    );
                    push_cloud_quad(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut indices,
                        [[x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [x0, y0, z0]],
                        [-1.0, 0.0, 0.0],
                        texel_x,
                        texel_z,
                        u_offset,
                        v_offset,
                    );
                    push_cloud_quad(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut indices,
                        [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
                        [1.0, 0.0, 0.0],
                        texel_x,
                        texel_z,
                        u_offset,
                        v_offset,
                    );
                    push_cloud_quad(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut indices,
                        [[x1, y1, z0], [x0, y1, z0], [x0, y0, z0], [x1, y0, z0]],
                        [0.0, 0.0, -1.0],
                        texel_x,
                        texel_z,
                        u_offset,
                        v_offset,
                    );
                    push_cloud_quad(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut indices,
                        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                        [0.0, 0.0, 1.0],
                        texel_x,
                        texel_z,
                        u_offset,
                        v_offset,
                    );
                }
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn push_cloud_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    texel_x: i32,
    texel_z: i32,
    u_offset: f32,
    v_offset: f32,
) {
    let base_index = u32::try_from(positions.len()).unwrap_or(u32::MAX - 4);
    let u = (texel_x as f32 + 0.5) * CLOUD_TEXEL_UV_SCALE + u_offset;
    let v = (texel_z as f32 + 0.5) * CLOUD_TEXEL_UV_SCALE + v_offset;

    for corner in corners {
        positions.push(corner);
        normals.push(normal);
        uvs.push([u, v]);
    }

    indices.extend_from_slice(&[
        base_index,
        base_index + 1,
        base_index + 2,
        base_index,
        base_index + 2,
        base_index + 3,
    ]);
}

fn seed_player_inventory(inventory: &mut lce_rust::world::PlayerInventory) {
    let _ = inventory.add_item(1, 64);
    let _ = inventory.add_item(2, 64);
    let _ = inventory.add_item(4, 64);
    let _ = inventory.add_item(5, 64);
}

fn world_vec3_to_bevy(value: lce_rust::world::Vec3) -> Vec3 {
    Vec3::new(value.x, value.y, value.z)
}

fn is_sword_item(item_id: u16) -> bool {
    SWORD_ITEM_IDS.contains(&item_id)
}

fn is_hand_equipped_item(item_id: u16) -> bool {
    SWORD_ITEM_IDS.contains(&item_id)
        || SHOVEL_ITEM_IDS.contains(&item_id)
        || PICKAXE_ITEM_IDS.contains(&item_id)
        || AXE_ITEM_IDS.contains(&item_id)
        || HOE_ITEM_IDS.contains(&item_id)
        || HAND_EQUIPPED_DIRECT_ITEM_IDS.contains(&item_id)
}

fn is_mirrored_art_item(item_id: u16) -> bool {
    MIRRORED_ART_ITEM_IDS.contains(&item_id)
}

fn is_eat_item(item_id: u16) -> bool {
    EAT_ITEM_IDS.contains(&item_id)
}

fn is_drink_item(item_id: u16) -> bool {
    DRINK_ITEM_IDS.contains(&item_id)
}

fn held_item_use_animation_for_item(item_id: u16) -> HeldItemUseAnimation {
    if is_sword_item(item_id) {
        HeldItemUseAnimation::Block
    } else if item_id == BOW_ITEM_ID {
        HeldItemUseAnimation::Bow
    } else if is_eat_item(item_id) || is_drink_item(item_id) {
        HeldItemUseAnimation::EatDrink
    } else {
        HeldItemUseAnimation::None
    }
}

fn use_animation_max_duration_ticks(use_animation: HeldItemUseAnimation) -> f32 {
    match use_animation {
        HeldItemUseAnimation::EatDrink => EAT_DRINK_USE_DURATION_TICKS,
        HeldItemUseAnimation::Block | HeldItemUseAnimation::Bow => {
            MAX_CONTINUOUS_USE_DURATION_TICKS
        }
        HeldItemUseAnimation::None => 0.0,
    }
}

fn is_pickaxe_item(item_id: u16) -> bool {
    PICKAXE_ITEM_IDS.contains(&item_id)
}

fn block_destroy_progress_per_tick(block_id: u16, selected_stack: Option<ItemStack>) -> f32 {
    if block_id == 0 || block_id == 7 {
        return 0.0;
    }

    if !is_solid_block_for_player_collision(block_id) {
        return 1.0;
    }

    let mut progress: f32 = if block_id == 49 {
        0.01
    } else if matches!(
        block_id,
        1 | 4 | 14 | 15 | 16 | 21 | 41 | 42 | 56 | 57 | 73 | 74 | 98 | 129 | 133
    ) {
        0.05
    } else {
        0.10
    };

    if let Some(stack) = selected_stack
        && is_pickaxe_item(stack.item_id)
        && matches!(
            block_id,
            1 | 4 | 14 | 15 | 16 | 21 | 41 | 42 | 49 | 56 | 57 | 73 | 74 | 98 | 129 | 133
        )
    {
        let tool_multiplier = match stack.item_id {
            270 => 1.4,
            274 => 2.2,
            257 => 3.0,
            278 => 3.6,
            285 => 6.0,
            _ => 1.0,
        };
        progress *= tool_multiplier;
    }

    progress.min(1.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlacementFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

fn placement_face_for_target(
    solid_hit: Option<BlockRaycastHit>,
    placement_target: BlockPos,
) -> Option<PlacementFace> {
    let hit = solid_hit?;
    if hit.adjacent_air_block != placement_target {
        return None;
    }

    let dx = placement_target.x - hit.block.x;
    let dy = placement_target.y - hit.block.y;
    let dz = placement_target.z - hit.block.z;

    match (dx, dy, dz) {
        (0, -1, 0) => Some(PlacementFace::Down),
        (0, 1, 0) => Some(PlacementFace::Up),
        (0, 0, -1) => Some(PlacementFace::North),
        (0, 0, 1) => Some(PlacementFace::South),
        (-1, 0, 0) => Some(PlacementFace::West),
        (1, 0, 0) => Some(PlacementFace::East),
        _ => None,
    }
}

fn block_placement_data(
    world: &BlockWorld,
    block_id: u16,
    placement_target: BlockPos,
    placement_face: Option<PlacementFace>,
    player_position: lce_rust::world::Vec3,
    player_yaw_radians: f32,
) -> Option<u8> {
    match block_id {
        50 | REDSTONE_TORCH_OFF_BLOCK_ID | REDSTONE_TORCH_ON_BLOCK_ID => Some(
            torch_placement_data(world, placement_target, placement_face),
        ),
        LEVER_BLOCK_ID => {
            lever_placement_data(world, placement_target, placement_face, player_yaw_radians)
        }
        29 | 33 => Some(piston_placement_facing(
            placement_target,
            player_position,
            player_yaw_radians,
        )),
        _ => None,
    }
}

fn torch_placement_data(
    world: &BlockWorld,
    placement_target: BlockPos,
    placement_face: Option<PlacementFace>,
) -> u8 {
    let mut dir = 0_u8;

    if let Some(face) = placement_face {
        if face == PlacementFace::Up
            && is_torch_connection_surface(
                world,
                BlockPos::new(
                    placement_target.x,
                    placement_target.y - 1,
                    placement_target.z,
                ),
            )
        {
            dir = 5;
        }
        if face == PlacementFace::North
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x,
                    placement_target.y,
                    placement_target.z + 1,
                ),
            )
        {
            dir = 4;
        }
        if face == PlacementFace::South
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x,
                    placement_target.y,
                    placement_target.z - 1,
                ),
            )
        {
            dir = 3;
        }
        if face == PlacementFace::West
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x + 1,
                    placement_target.y,
                    placement_target.z,
                ),
            )
        {
            dir = 2;
        }
        if face == PlacementFace::East
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x - 1,
                    placement_target.y,
                    placement_target.z,
                ),
            )
        {
            dir = 1;
        }
    }

    if dir == 0 {
        if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x - 1,
                placement_target.y,
                placement_target.z,
            ),
        ) {
            dir = 1;
        } else if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x + 1,
                placement_target.y,
                placement_target.z,
            ),
        ) {
            dir = 2;
        } else if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x,
                placement_target.y,
                placement_target.z - 1,
            ),
        ) {
            dir = 3;
        } else if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x,
                placement_target.y,
                placement_target.z + 1,
            ),
        ) {
            dir = 4;
        } else if is_torch_connection_surface(
            world,
            BlockPos::new(
                placement_target.x,
                placement_target.y - 1,
                placement_target.z,
            ),
        ) {
            dir = 5;
        }
    }

    dir
}

fn lever_placement_data(
    world: &BlockWorld,
    placement_target: BlockPos,
    placement_face: Option<PlacementFace>,
    player_yaw_radians: f32,
) -> Option<u8> {
    let mut dir = None;

    if let Some(face) = placement_face {
        if face == PlacementFace::Down
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x,
                    placement_target.y + 1,
                    placement_target.z,
                ),
            )
        {
            dir = Some(0_i32);
        }
        if face == PlacementFace::Up
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x,
                    placement_target.y - 1,
                    placement_target.z,
                ),
            )
        {
            dir = Some(5_i32);
        }
        if face == PlacementFace::North
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x,
                    placement_target.y,
                    placement_target.z + 1,
                ),
            )
        {
            dir = Some(4_i32);
        }
        if face == PlacementFace::South
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x,
                    placement_target.y,
                    placement_target.z - 1,
                ),
            )
        {
            dir = Some(3_i32);
        }
        if face == PlacementFace::West
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x + 1,
                    placement_target.y,
                    placement_target.z,
                ),
            )
        {
            dir = Some(2_i32);
        }
        if face == PlacementFace::East
            && is_support_solid(
                world,
                BlockPos::new(
                    placement_target.x - 1,
                    placement_target.y,
                    placement_target.z,
                ),
            )
        {
            dir = Some(1_i32);
        }
    }

    if dir.is_none() {
        if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x - 1,
                placement_target.y,
                placement_target.z,
            ),
        ) {
            dir = Some(1);
        } else if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x + 1,
                placement_target.y,
                placement_target.z,
            ),
        ) {
            dir = Some(2);
        } else if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x,
                placement_target.y,
                placement_target.z - 1,
            ),
        ) {
            dir = Some(3);
        } else if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x,
                placement_target.y,
                placement_target.z + 1,
            ),
        ) {
            dir = Some(4);
        } else if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x,
                placement_target.y - 1,
                placement_target.z,
            ),
        ) {
            dir = Some(5);
        } else if is_support_solid(
            world,
            BlockPos::new(
                placement_target.x,
                placement_target.y + 1,
                placement_target.z,
            ),
        ) {
            dir = Some(0);
        }
    }

    let mut dir = dir?;
    let yaw_even =
        (((player_yaw_radians.to_degrees() * 4.0 / 360.0) + 0.5).floor() as i32 & 1) == 0;
    if dir == 5 {
        dir = if yaw_even { 5 } else { 6 };
    } else if dir == 0 {
        dir = if yaw_even { 7 } else { 0 };
    }

    Some(dir as u8)
}

fn piston_placement_facing(
    placement_target: BlockPos,
    player_position: lce_rust::world::Vec3,
    player_yaw_radians: f32,
) -> u8 {
    if (player_position.x - placement_target.x as f32).abs() < 2.0
        && (player_position.z - placement_target.z as f32).abs() < 2.0
    {
        let player_y = player_position.y + 0.2;
        if player_y - placement_target.y as f32 > 2.0 {
            return 1;
        }
        if placement_target.y as f32 - player_y > 0.0 {
            return 0;
        }
    }

    let i = ((player_yaw_radians.to_degrees() * 4.0 / 360.0) + 0.5).floor() as i32 & 0x3;
    match i {
        0 => 2,
        1 => 5,
        2 => 3,
        _ => 4,
    }
}

fn is_torch_connection_surface(world: &BlockWorld, block: BlockPos) -> bool {
    let block_id = world.block_id(block);
    is_support_solid(world, block)
        || block_id == FENCE_BLOCK_ID
        || block_id == NETHER_FENCE_BLOCK_ID
        || block_id == GLASS_BLOCK_ID
        || block_id == COBBLE_WALL_BLOCK_ID
}

fn is_support_solid(world: &BlockWorld, block: BlockPos) -> bool {
    is_solid_block_for_player_collision(world.block_id(block))
}

fn default_present_mode() -> bevy::window::PresentMode {
    if let Ok(value) = std::env::var("LCE_VSYNC") {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized == "0" || normalized == "false" || normalized == "off" {
            return bevy::window::PresentMode::AutoNoVsync;
        }
        if normalized == "1" || normalized == "true" || normalized == "on" {
            return bevy::window::PresentMode::AutoVsync;
        }
    }

    bevy::window::PresentMode::AutoVsync
}

fn build_block_materials(
    asset_server: &AssetServer,
    runtime_assets: &RuntimeAssets,
) -> (StandardMaterial, StandardMaterial, Option<Handle<Image>>) {
    if let Some(texture_path) = runtime_assets.0.terrain_texture_asset_path.as_ref() {
        let texture = asset_server.load(texture_path.clone());
        return (
            StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(texture.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                cull_mode: None,
                ..default()
            },
            StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(texture.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            },
            Some(texture),
        );
    }

    (
        StandardMaterial {
            base_color: Color::srgb(0.42, 0.64, 0.33),
            unlit: true,
            alpha_mode: AlphaMode::Opaque,
            cull_mode: None,
            ..default()
        },
        StandardMaterial {
            base_color: Color::srgb(0.42, 0.64, 0.33),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        },
        None,
    )
}

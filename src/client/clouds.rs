use crate::world::{WATER_FLOWING_BLOCK_ID, WATER_SOURCE_BLOCK_ID};

pub const CLOUD_TILE_SIZE: f32 = 32.0;
pub const CLOUD_TILE_REPEAT_RADIUS: i32 = 8;
pub const CLOUD_TEXTURE_WRAP_BLOCKS: f64 = 2048.0;
pub const CLOUD_UV_SCALE: f32 = 1.0 / 2048.0;
pub const CLOUD_SCROLL_PER_TICK: f64 = 0.03;
pub const CLOUD_HEIGHT: f32 = 128.0;
pub const CLOUD_LAYER_THICKNESS: f32 = 4.0;
pub const CLOUD_Y_CAMERA_OFFSET: f32 = 0.33;
pub const CLOUD_ALPHA: f32 = 0.8;

pub fn cloud_tick_time(total_ticks: u64, tick_alpha: f32) -> f64 {
    total_ticks as f64 + f64::from(tick_alpha.clamp(0.0, 1.0))
}

pub fn cloud_uv_offset(player_x: f64, player_z: f64, tick_time: f64) -> (f32, f32) {
    let mut xo = player_x + tick_time * CLOUD_SCROLL_PER_TICK;
    let mut zo = player_z;

    xo -= (xo / CLOUD_TEXTURE_WRAP_BLOCKS).floor() * CLOUD_TEXTURE_WRAP_BLOCKS;
    zo -= (zo / CLOUD_TEXTURE_WRAP_BLOCKS).floor() * CLOUD_TEXTURE_WRAP_BLOCKS;

    ((xo as f32) * CLOUD_UV_SCALE, (zo as f32) * CLOUD_UV_SCALE)
}

pub fn cloud_camera_relative_y(camera_player_y: f32) -> f32 {
    CLOUD_HEIGHT - camera_player_y + CLOUD_Y_CAMERA_OFFSET
}

pub fn cloud_world_y(camera_player_y: f32) -> f32 {
    camera_player_y + cloud_camera_relative_y(camera_player_y)
}

pub fn clouds_visible_for_camera_block(block_id: u16) -> bool {
    block_id != WATER_SOURCE_BLOCK_ID && block_id != WATER_FLOWING_BLOCK_ID
}

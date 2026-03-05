use crate::world::{WATER_FLOWING_BLOCK_ID, WATER_SOURCE_BLOCK_ID};

pub const CLOUD_ADVANCED_TEXELS_PER_SECTION: i32 = 8;
pub const CLOUD_ADVANCED_SECTION_RADIUS: i32 = 3;
pub const CLOUD_ADVANCED_TEXEL_WORLD_SIZE: f32 = 12.0;
pub const CLOUD_TEXTURE_SIZE_TEXELS: f32 = 256.0;
pub const CLOUD_TEXEL_UV_SCALE: f32 = 1.0 / CLOUD_TEXTURE_SIZE_TEXELS;
pub const CLOUD_TEXTURE_WRAP_TEXELS: f64 = 2048.0;
pub const CLOUD_SCROLL_PER_TICK: f64 = 0.03;
pub const CLOUD_HEIGHT: f32 = 128.0;
pub const CLOUD_LAYER_THICKNESS: f32 = 4.0;
pub const CLOUD_Y_CAMERA_OFFSET: f32 = 0.33;
pub const CLOUD_ALPHA: f32 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudUvMotion {
    pub u_offset: f32,
    pub v_offset: f32,
    pub x_offset_blocks: f32,
    pub z_offset_blocks: f32,
}

pub fn cloud_tick_time(total_ticks: u64, tick_alpha: f32) -> f64 {
    total_ticks as f64 + f64::from(tick_alpha.clamp(0.0, 1.0))
}

pub fn cloud_uv_motion(player_x: f64, player_z: f64, tick_time: f64) -> CloudUvMotion {
    let world_scale = f64::from(CLOUD_ADVANCED_TEXEL_WORLD_SIZE);
    let mut xo = (player_x + tick_time * CLOUD_SCROLL_PER_TICK) / world_scale;
    let mut zo = player_z / world_scale + 0.33;

    xo -= (xo / CLOUD_TEXTURE_WRAP_TEXELS).floor() * CLOUD_TEXTURE_WRAP_TEXELS;
    zo -= (zo / CLOUD_TEXTURE_WRAP_TEXELS).floor() * CLOUD_TEXTURE_WRAP_TEXELS;

    let mut u_offset = xo.floor() * f64::from(CLOUD_TEXEL_UV_SCALE);
    let mut v_offset = zo.floor() * f64::from(CLOUD_TEXEL_UV_SCALE);

    while u_offset < 1.0 {
        u_offset += 1.0;
    }
    while v_offset < 1.0 {
        v_offset += 1.0;
    }

    let x_offset_blocks = ((xo - xo.floor()) * world_scale) as f32;
    let z_offset_blocks = ((zo - zo.floor()) * world_scale) as f32;

    CloudUvMotion {
        u_offset: u_offset as f32,
        v_offset: v_offset as f32,
        x_offset_blocks,
        z_offset_blocks,
    }
}

pub fn cloud_uv_offset(player_x: f64, player_z: f64, tick_time: f64) -> (f32, f32) {
    let motion = cloud_uv_motion(player_x, player_z, tick_time);
    (motion.u_offset, motion.v_offset)
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

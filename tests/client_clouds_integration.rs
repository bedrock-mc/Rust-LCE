use lce_rust::client::clouds::{
    CLOUD_HEIGHT, CLOUD_LAYER_THICKNESS, CLOUD_Y_CAMERA_OFFSET, cloud_camera_relative_y,
    cloud_tick_time, cloud_uv_motion, cloud_uv_offset, cloud_world_y,
    clouds_visible_for_camera_block,
};
use lce_rust::world::{WATER_FLOWING_BLOCK_ID, WATER_SOURCE_BLOCK_ID};

#[test]
fn cloud_tick_time_clamps_partial_tick_alpha() {
    let clamped_high = cloud_tick_time(120, 1.4);
    let clamped_low = cloud_tick_time(120, -0.5);
    let nominal = cloud_tick_time(120, 0.25);

    assert_eq!(clamped_high, 121.0);
    assert_eq!(clamped_low, 120.0);
    assert_eq!(nominal, 120.25);
}

#[test]
fn cloud_uv_offset_matches_legacy_advanced_wrap_and_scroll_formula() {
    let tick_time = cloud_tick_time(1_000, 0.25);
    let motion = cloud_uv_motion(3_000.0, -10.0, tick_time);
    let (u, v) = cloud_uv_offset(3_000.0, -10.0, tick_time);

    assert!(approx_eq(motion.u_offset, 1.984_375));
    assert!(approx_eq(motion.v_offset, 7.996_093_8));
    assert!(approx_eq(motion.x_offset_blocks, 6.007_5));
    assert!(approx_eq(motion.z_offset_blocks, 5.96));
    assert!(approx_eq(u, motion.u_offset));
    assert!(approx_eq(v, motion.v_offset));
}

#[test]
fn cloud_relative_y_matches_legacy_camera_relative_height() {
    let y = cloud_camera_relative_y(70.0);
    let expected = CLOUD_HEIGHT - 70.0 + CLOUD_Y_CAMERA_OFFSET;
    assert!(approx_eq(y, expected));
}

#[test]
fn cloud_world_y_is_stable_at_legacy_cloud_height() {
    let y_low = cloud_world_y(0.0);
    let y_high = cloud_world_y(90.0);
    let expected = CLOUD_HEIGHT + CLOUD_Y_CAMERA_OFFSET;

    assert!(approx_eq(y_low, expected));
    assert!(approx_eq(y_high, expected));
}

#[test]
fn clouds_hide_when_camera_block_is_water() {
    assert!(clouds_visible_for_camera_block(0));
    assert!(!clouds_visible_for_camera_block(WATER_SOURCE_BLOCK_ID));
    assert!(!clouds_visible_for_camera_block(WATER_FLOWING_BLOCK_ID));
}

#[test]
fn cloud_layer_thickness_matches_legacy_advanced_height() {
    assert!(approx_eq(CLOUD_LAYER_THICKNESS, 4.0));
}

fn approx_eq(left: f32, right: f32) -> bool {
    (left - right).abs() <= 1e-5
}

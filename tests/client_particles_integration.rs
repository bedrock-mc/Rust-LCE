use lce_rust::client::particles::terrain_break_particle_tile;
use lce_rust::client::terrain_meshing::{BlockFace, atlas_tile_for_block_face};

#[test]
fn terrain_break_particles_use_bottom_face_texture_for_grass() {
    let particle_tile = terrain_break_particle_tile(2, 0);
    assert_eq!(
        particle_tile,
        atlas_tile_for_block_face(2, BlockFace::Bottom)
    );
    assert_ne!(particle_tile, atlas_tile_for_block_face(2, BlockFace::Top));
}

#[test]
fn terrain_break_particles_follow_bottom_face_for_general_blocks() {
    assert_eq!(
        terrain_break_particle_tile(1, 0),
        atlas_tile_for_block_face(1, BlockFace::Bottom)
    );
    assert_eq!(
        terrain_break_particle_tile(17, 2),
        atlas_tile_for_block_face(17, BlockFace::Bottom)
    );
}

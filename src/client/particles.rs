use crate::client::terrain_meshing::{BlockFace, atlas_tile_for_block_face};

pub fn terrain_break_particle_tile(block_id: u16, _block_aux: u16) -> (u8, u8) {
    atlas_tile_for_block_face(block_id, BlockFace::Bottom)
}

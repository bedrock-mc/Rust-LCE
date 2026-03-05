use lce_rust::client::terrain_meshing::{
    BlockFace, atlas_tile_for_block_face, build_chunk_mesh_data, dirty_chunks_for_block,
};
use lce_rust::world::{
    BlockPos, BlockWorld, ChunkPos, REDSTONE_WIRE_BLOCK_ID, REDSTONE_WIRE_POWERED_BLOCK_ID,
    WATER_SOURCE_BLOCK_ID,
};

#[test]
fn empty_chunk_produces_no_mesh() {
    let world = BlockWorld::new();
    assert!(build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).is_none());
}

#[test]
fn single_block_mesh_has_six_faces() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 0), 1);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    assert_eq!(mesh.positions.len(), 24);
    assert_eq!(mesh.normals.len(), 24);
    assert_eq!(mesh.uvs.len(), 24);
    assert_eq!(mesh.colors.len(), 24);
    assert_eq!(mesh.indices.len(), 36);
}

#[test]
fn adjacent_blocks_cull_internal_faces() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 0), 1);
    world.place_block(BlockPos::new(1, 64, 0), 1);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    assert_eq!(mesh.positions.len(), 40);
    assert_eq!(mesh.indices.len(), 60);
}

#[test]
fn adjacent_blocks_across_chunk_boundary_cull_internal_faces() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(15, 64, 0), 1);
    world.place_block(BlockPos::new(16, 64, 0), 1);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    assert_eq!(mesh.positions.len(), 20);
    assert_eq!(mesh.indices.len(), 30);
}

#[test]
fn dirty_chunks_include_neighbors_for_boundary_blocks() {
    let dirty = dirty_chunks_for_block(BlockPos::new(15, 70, 0));

    assert!(dirty.contains(&ChunkPos::new(0, 0)));
    assert!(dirty.contains(&ChunkPos::new(1, 0)));
    assert!(dirty.contains(&ChunkPos::new(0, -1)));
}

#[test]
fn dirty_chunks_include_negative_neighbors_for_origin_boundary_blocks() {
    let dirty = dirty_chunks_for_block(BlockPos::new(0, 70, 0));

    assert!(dirty.contains(&ChunkPos::new(0, 0)));
    assert!(dirty.contains(&ChunkPos::new(-1, 0)));
    assert!(dirty.contains(&ChunkPos::new(0, -1)));
}

#[test]
fn grass_block_uses_expected_top_and_side_tiles() {
    assert_eq!(atlas_tile_for_block_face(2, BlockFace::Top), (0, 0));
    assert_eq!(atlas_tile_for_block_face(2, BlockFace::North), (3, 0));
}

#[test]
fn water_block_uses_water_tile_instead_of_stone_fallback() {
    assert_eq!(
        atlas_tile_for_block_face(WATER_SOURCE_BLOCK_ID, BlockFace::Top),
        (13, 12)
    );
    assert_eq!(
        atlas_tile_for_block_face(WATER_SOURCE_BLOCK_ID, BlockFace::North),
        (14, 12)
    );
}

#[test]
fn side_face_uvs_keep_vertical_axis_on_v_coordinate() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 0), 2);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    let north_uvs = &mesh.uvs[8..12];
    assert!(north_uvs[0][1] > north_uvs[1][1]);
    assert!(north_uvs[3][1] > north_uvs[2][1]);
}

#[test]
fn atlas_uvs_are_inset_from_tile_edges_to_reduce_texture_bleed() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 0), 2);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    let top_uvs = &mesh.uvs[0..4];
    let (min_u, max_u) = top_uvs
        .iter()
        .map(|uv| uv[0])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min_u, max_u), u| {
            (min_u.min(u), max_u.max(u))
        });

    let tile = 1.0 / 16.0;
    assert!(min_u > 0.0);
    assert!(max_u < tile);
}

#[test]
fn grass_top_vertices_are_tinted_not_flat_grayscale() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 0), 2);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    let top_colors = &mesh.colors[0..4];
    let expected = [124.0 / 255.0, 189.0 / 255.0, 107.0 / 255.0, 1.0];

    for color in top_colors {
        assert!((color[0] - expected[0]).abs() < 0.0001);
        assert!((color[1] - expected[1]).abs() < 0.0001);
        assert!((color[2] - expected[2]).abs() < 0.0001);
        assert!((color[3] - expected[3]).abs() < 0.0001);
    }
}

#[test]
fn flowing_water_mesh_has_lower_top_surface_height() {
    let mut world = BlockWorld::new();
    let water = BlockPos::new(0, 64, 0);
    world.place_block(water, WATER_SOURCE_BLOCK_ID);
    world.set_block_data(water, 8);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    let top_y_values = mesh
        .positions
        .iter()
        .filter(|position| position[1] > 64.0 && position[1] < 65.0)
        .map(|position| position[1])
        .collect::<Vec<_>>();

    assert!(!top_y_values.is_empty());
    assert!(top_y_values.iter().all(|y| *y < 64.95));
}

#[test]
fn redstone_wire_uses_redstone_tiles_instead_of_stone_fallback() {
    assert_eq!(
        atlas_tile_for_block_face(REDSTONE_WIRE_BLOCK_ID, BlockFace::Top),
        (4, 10)
    );
    assert_eq!(
        atlas_tile_for_block_face(REDSTONE_WIRE_BLOCK_ID, BlockFace::North),
        (4, 10)
    );
    assert_eq!(
        atlas_tile_for_block_face(REDSTONE_WIRE_POWERED_BLOCK_ID, BlockFace::Top),
        (4, 10)
    );
}

#[test]
fn redstone_wire_mesh_renders_as_flat_top_quad_not_cube() {
    let mut world = BlockWorld::new();
    world.place_block(BlockPos::new(0, 64, 0), REDSTONE_WIRE_BLOCK_ID);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    assert_eq!(mesh.positions.len(), 8);
    assert_eq!(mesh.indices.len(), 12);

    let y_values = mesh.positions.iter().map(|position| position[1]);
    assert!(y_values.into_iter().all(|y| y > 64.0 && y < 65.0));

    let min_x = mesh
        .positions
        .iter()
        .map(|position| position[0])
        .fold(f32::INFINITY, f32::min);
    let max_x = mesh
        .positions
        .iter()
        .map(|position| position[0])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(min_x > 0.0);
    assert!(max_x < 1.0);
}

#[test]
fn quartz_block_id_uses_quartz_tiles_not_redstone_internal_powered_id() {
    assert_eq!(
        atlas_tile_for_block_face(155, BlockFace::Top),
        (11, 13),
        "quartz block top should map to quartz atlas tile"
    );
    assert_eq!(
        atlas_tile_for_block_face(155, BlockFace::North),
        (11, 14),
        "quartz block side should map to quartz side tile"
    );

    assert_eq!(
        atlas_tile_for_block_face(REDSTONE_WIRE_POWERED_BLOCK_ID, BlockFace::Top),
        (4, 10),
        "internal powered redstone wire id should still map to redstone base texture"
    );
}

#[test]
fn redstone_wire_mesh_applies_lit_and_unlit_tints() {
    let mut unlit_world = BlockWorld::new();
    unlit_world.place_block(BlockPos::new(0, 64, 0), REDSTONE_WIRE_BLOCK_ID);

    let unlit_mesh =
        build_chunk_mesh_data(&unlit_world, ChunkPos::new(0, 0)).expect("mesh should be generated");
    assert_eq!(unlit_mesh.colors[0], [76.0 / 255.0, 0.0, 0.0, 1.0]);

    let mut lit_world = BlockWorld::new();
    lit_world.place_block(BlockPos::new(0, 64, 0), REDSTONE_WIRE_POWERED_BLOCK_ID);

    let lit_mesh =
        build_chunk_mesh_data(&lit_world, ChunkPos::new(0, 0)).expect("mesh should be generated");
    assert_eq!(lit_mesh.colors[0], [1.0, 50.0 / 255.0, 0.0, 1.0]);
}

#[test]
fn adjacent_redstone_wires_extend_toward_connected_neighbor() {
    let mut isolated_world = BlockWorld::new();
    isolated_world.place_block(BlockPos::new(0, 64, 0), REDSTONE_WIRE_BLOCK_ID);
    let isolated = build_chunk_mesh_data(&isolated_world, ChunkPos::new(0, 0))
        .expect("mesh should be generated");
    let isolated_max_x = isolated
        .positions
        .iter()
        .map(|position| position[0])
        .fold(f32::NEG_INFINITY, f32::max);

    let mut connected_world = BlockWorld::new();
    connected_world.place_block(BlockPos::new(0, 64, 0), REDSTONE_WIRE_BLOCK_ID);
    connected_world.place_block(BlockPos::new(1, 64, 0), REDSTONE_WIRE_BLOCK_ID);
    let connected = build_chunk_mesh_data(&connected_world, ChunkPos::new(0, 0))
        .expect("mesh should be generated");
    let connected_max_x = connected
        .positions
        .iter()
        .filter(|position| position[0] <= 1.0)
        .map(|position| position[0])
        .fold(f32::NEG_INFINITY, f32::max);

    assert!(isolated_max_x < 1.0);
    assert!(
        (connected_max_x - 1.0).abs() < 0.0001,
        "wire should extend to shared block edge when connected"
    );
}

#[test]
fn torch_and_lever_meshes_are_not_full_cubes() {
    let mut world = BlockWorld::new();

    let torch = BlockPos::new(0, 64, 0);
    world.place_block(torch, 50);
    world.set_block_data(torch, 5);

    let lever = BlockPos::new(2, 64, 0);
    world.place_block(lever, 69);
    world.set_block_data(lever, 5);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    let torch_x = mesh
        .positions
        .iter()
        .filter(|position| position[0] >= 0.0 && position[0] <= 1.0)
        .map(|position| position[0])
        .collect::<Vec<_>>();
    let lever_x = mesh
        .positions
        .iter()
        .filter(|position| position[0] >= 2.0 && position[0] <= 3.0)
        .map(|position| position[0])
        .collect::<Vec<_>>();

    let torch_width = torch_x.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
        - torch_x.iter().cloned().fold(f32::INFINITY, f32::min);
    let lever_width = lever_x.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
        - lever_x.iter().cloned().fold(f32::INFINITY, f32::min);

    assert!(torch_width < 0.35);
    assert!(lever_width < 0.6);
}

#[test]
fn piston_mesh_respects_block_data_facing_for_front_texture() {
    let mut world = BlockWorld::new();
    let piston = BlockPos::new(0, 64, 0);
    world.place_block(piston, 33);
    world.set_block_data(piston, 5);

    let mesh =
        build_chunk_mesh_data(&world, ChunkPos::new(0, 0)).expect("mesh should be generated");

    let top_uvs = mesh
        .positions
        .iter()
        .zip(mesh.normals.iter())
        .zip(mesh.uvs.iter())
        .filter(|((position, normal), _)| {
            (normal[1] - 1.0).abs() < 0.0001 && (position[1] - 65.0).abs() < 0.0001
        })
        .map(|((_, _), uv)| *uv)
        .collect::<Vec<_>>();
    assert!(!top_uvs.is_empty());
    assert!(top_uvs.iter().all(|uv| uv_in_tile(*uv, 12, 6)));

    let east_uvs = mesh
        .positions
        .iter()
        .zip(mesh.normals.iter())
        .zip(mesh.uvs.iter())
        .filter(|((position, normal), _)| {
            (normal[0] - 1.0).abs() < 0.0001 && (position[0] - 1.0).abs() < 0.0001
        })
        .map(|((_, _), uv)| *uv)
        .collect::<Vec<_>>();
    assert!(!east_uvs.is_empty());
    assert!(east_uvs.iter().all(|uv| uv_in_tile(*uv, 11, 6)));
}

fn uv_in_tile(uv: [f32; 2], tile_x: u8, tile_y: u8) -> bool {
    let tile = 1.0 / 16.0;
    let inset = 0.001;
    let u_min = f32::from(tile_x) * tile + inset;
    let u_max = (f32::from(tile_x) + 1.0) * tile - inset;
    let v_min = f32::from(tile_y) * tile + inset;
    let v_max = (f32::from(tile_y) + 1.0) * tile - inset;

    uv[0] >= u_min - 0.0001
        && uv[0] <= u_max + 0.0001
        && uv[1] >= v_min - 0.0001
        && uv[1] <= v_max + 0.0001
}

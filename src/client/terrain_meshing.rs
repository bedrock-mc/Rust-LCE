use crate::world::{
    BlockPos, BlockWorld, ChunkPos, FluidKind, LAVA_FLOWING_BLOCK_ID, LAVA_SOURCE_BLOCK_ID,
    REDSTONE_WIRE_BLOCK_ID, REDSTONE_WIRE_POWERED_BLOCK_ID, WATER_FLOWING_BLOCK_ID,
    WATER_SOURCE_BLOCK_ID, fluid_kind_for_block, is_fluid_block, is_redstone_block,
};

pub const TERRAIN_ATLAS_TILES: u8 = 16;
const ATLAS_UV_INSET: f32 = 0.001;
const GRASS_TOP_TINT: [f32; 4] = [124.0 / 255.0, 189.0 / 255.0, 107.0 / 255.0, 1.0];
const WATER_TINT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const LAVA_TINT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const FLUID_FACE_OFFSET: f32 = 0.001;
const FARMLAND_BLOCK_ID: u16 = 60;
const REDSTONE_WIRE_RENDER_OFFSET: f32 = 0.015625;
const FLAT_TILE_RENDER_OFFSET: f32 = 0.015625;
const REDSTONE_WIRE_UNLIT_TINT: [f32; 4] = [76.0 / 255.0, 0.0, 0.0, 1.0];
const REDSTONE_WIRE_LIT_TINT: [f32; 4] = [1.0, 50.0 / 255.0, 0.0, 1.0];
const REDSTONE_WIRE_CUT_DISTANCE: f32 = 5.0 / 16.0;
const BREAK_OVERLAY_ALPHA: f32 = 0.5;
const BREAK_OVERLAY_NORMAL_OFFSET: f32 = 0.002;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFace {
    Top,
    Bottom,
    North,
    South,
    West,
    East,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TerrainMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub face_render_layer: Vec<u8>,
}

pub fn build_chunk_mesh_data(world: &BlockWorld, chunk: ChunkPos) -> Option<TerrainMeshData> {
    if world.chunk_block_count(chunk) == 0 {
        return None;
    }

    let mut mesh = TerrainMeshData::default();

    world.for_each_block_in_chunk(chunk, |block_pos, block_id| {
        if is_fluid_block(block_id) {
            append_fluid_block(world, &mut mesh, block_pos, block_id);
            return;
        }

        if block_id == REDSTONE_WIRE_BLOCK_ID || block_id == REDSTONE_WIRE_POWERED_BLOCK_ID {
            append_redstone_wire(world, &mut mesh, block_pos, block_id);
            return;
        }

        if is_torch_block(block_id) {
            append_torch(world, &mut mesh, block_pos, block_id);
            return;
        }

        if block_id == 69 {
            append_lever(world, &mut mesh, block_pos, block_id);
            return;
        }

        if block_id == 29 || block_id == 33 {
            append_oriented_piston_block(world, &mut mesh, block_pos, block_id);
            return;
        }

        if let Some((tile_x, tile_y)) = flat_top_block_tile(block_id) {
            append_flat_tile(
                &mut mesh,
                block_pos,
                tile_x,
                tile_y,
                FLAT_TILE_RENDER_OFFSET,
                block_render_layer(block_id),
            );
            return;
        }

        if let Some((tile_x, tile_y)) = cross_plane_block_tile(block_id) {
            append_cross_plane_block(
                &mut mesh,
                block_pos,
                tile_x,
                tile_y,
                block_render_layer(block_id),
            );
            return;
        }

        for face in FACE_DEFS {
            let neighbor = BlockPos::new(
                block_pos.x + face.neighbor[0],
                block_pos.y + face.neighbor[1],
                block_pos.z + face.neighbor[2],
            );

            let neighbor_block = world.block_id(neighbor);
            if is_face_occluding_block(neighbor_block) {
                continue;
            }

            append_face(&mut mesh, block_pos, block_id, face);
        }
    });

    if mesh.indices.is_empty() {
        None
    } else {
        Some(mesh)
    }
}

pub fn build_block_break_overlay_mesh_data(
    world: &BlockWorld,
    block_pos: BlockPos,
    stage: u8,
) -> Option<TerrainMeshData> {
    let block_id = world.block_id(block_pos);
    if block_id == 0 || is_fluid_block(block_id) {
        return None;
    }

    let mut mesh = TerrainMeshData::default();

    if block_id == REDSTONE_WIRE_BLOCK_ID || block_id == REDSTONE_WIRE_POWERED_BLOCK_ID {
        append_redstone_wire(world, &mut mesh, block_pos, block_id);
    } else if is_torch_block(block_id) {
        append_torch(world, &mut mesh, block_pos, block_id);
    } else if block_id == 69 {
        append_lever(world, &mut mesh, block_pos, block_id);
    } else if block_id == 29 || block_id == 33 {
        append_oriented_piston_block(world, &mut mesh, block_pos, block_id);
    } else if let Some((tile_x, tile_y)) = flat_top_block_tile(block_id) {
        append_flat_tile(
            &mut mesh,
            block_pos,
            tile_x,
            tile_y,
            FLAT_TILE_RENDER_OFFSET,
            block_render_layer(block_id),
        );
    } else if let Some((tile_x, tile_y)) = cross_plane_block_tile(block_id) {
        append_cross_plane_block(
            &mut mesh,
            block_pos,
            tile_x,
            tile_y,
            block_render_layer(block_id),
        );
    } else {
        for face in FACE_DEFS {
            append_face(&mut mesh, block_pos, block_id, face);
        }
    }

    if mesh.indices.is_empty() {
        return None;
    }

    let uv = atlas_uv(stage.min(9), 15);
    let face_count = mesh.positions.len() / 4;
    for face_index in 0..face_count {
        let vertex_start = face_index * 4;

        mesh.uvs[vertex_start..vertex_start + 4].copy_from_slice(&uv);

        for vertex_index in vertex_start..vertex_start + 4 {
            mesh.colors[vertex_index] = [0.0, 0.0, 0.0, BREAK_OVERLAY_ALPHA];

            let normal = mesh.normals[vertex_index];
            mesh.positions[vertex_index][0] += normal[0] * BREAK_OVERLAY_NORMAL_OFFSET;
            mesh.positions[vertex_index][1] += normal[1] * BREAK_OVERLAY_NORMAL_OFFSET;
            mesh.positions[vertex_index][2] += normal[2] * BREAK_OVERLAY_NORMAL_OFFSET;
        }
    }

    mesh.face_render_layer.resize(face_count, 1);

    Some(mesh)
}

pub fn dirty_chunks_for_block(block_pos: BlockPos) -> Vec<ChunkPos> {
    let center = ChunkPos::from_block(block_pos);
    let mut chunks = Vec::with_capacity(5);
    chunks.push(center);

    let local_x = block_pos.x.rem_euclid(16);
    let local_z = block_pos.z.rem_euclid(16);

    if local_x == 0 {
        chunks.push(ChunkPos::new(center.x - 1, center.z));
    }
    if local_x == 15 {
        chunks.push(ChunkPos::new(center.x + 1, center.z));
    }
    if local_z == 0 {
        chunks.push(ChunkPos::new(center.x, center.z - 1));
    }
    if local_z == 15 {
        chunks.push(ChunkPos::new(center.x, center.z + 1));
    }

    chunks
}

pub fn atlas_tile_for_block_face(block_id: u16, face: BlockFace) -> (u8, u8) {
    match block_id {
        1 => (1, 0),
        2 => match face {
            BlockFace::Top => (0, 0),
            BlockFace::Bottom => (2, 0),
            _ => (3, 0),
        },
        3 => (2, 0),
        4 => (0, 1),
        5 => (4, 0),
        6 => (15, 0),
        7 => (1, 1),
        12 => (2, 1),
        13 => (3, 1),
        14 => (0, 2),
        15 => (1, 2),
        16 => (2, 2),
        17 => match face {
            BlockFace::Top | BlockFace::Bottom => (5, 1),
            _ => (4, 1),
        },
        18 => (4, 3),
        19 => (0, 3),
        20 => (1, 3),
        21 => (0, 10),
        22 => (0, 9),
        23 => match face {
            BlockFace::North => (14, 2),
            _ => (1, 0),
        },
        24 => match face {
            BlockFace::Top => (0, 11),
            BlockFace::Bottom => (0, 13),
            _ => (0, 12),
        },
        25 => (10, 4),
        26 => match face {
            BlockFace::Top => (6, 8),
            BlockFace::Bottom => (4, 0),
            _ => (6, 9),
        },
        27 => (3, 10),
        28 => (3, 12),
        29 => match face {
            BlockFace::Top => (10, 6),
            BlockFace::Bottom => (13, 6),
            _ => (12, 6),
        },
        30 => (11, 0),
        31 => (7, 2),
        32 => (7, 3),
        33 => match face {
            BlockFace::Top => (11, 6),
            BlockFace::Bottom => (13, 6),
            _ => (12, 6),
        },
        35 => (0, 4),
        37 => (13, 0),
        38 => (12, 0),
        39 => (13, 1),
        40 => (12, 1),
        41 => (7, 1),
        42 => (6, 1),
        44 => match face {
            BlockFace::Top | BlockFace::Bottom => (6, 0),
            _ => (5, 0),
        },
        45 => (7, 0),
        48 => (4, 2),
        49 => (5, 2),
        50 => (0, 5),
        53 => (4, 0),
        54 => (4, 0),
        57 => (8, 1),
        58 => match face {
            BlockFace::Top => (11, 3),
            BlockFace::Bottom => (4, 0),
            _ => (12, 3),
        },
        59 => (15, 5),
        60 => match face {
            BlockFace::Top => (7, 5),
            _ => (2, 0),
        },
        61 => match face {
            BlockFace::Top | BlockFace::Bottom => (5, 3),
            BlockFace::North => (12, 2),
            _ => (13, 2),
        },
        62 => match face {
            BlockFace::Top | BlockFace::Bottom => (14, 3),
            BlockFace::North => (13, 3),
            _ => (13, 2),
        },
        63 => (4, 0),
        64 => (1, 5),
        65 => (3, 5),
        66 => (0, 8),
        67 => (0, 1),
        68 => (4, 0),
        69 => (0, 6),
        70 => (1, 0),
        71 => (2, 5),
        72 => (4, 0),
        73 | 74 => (3, 3),
        75 => (3, 7),
        76 => (3, 6),
        77 => (1, 0),
        78 => match face {
            BlockFace::Top => (2, 4),
            _ => (4, 4),
        },
        79 => (3, 4),
        80 => (2, 4),
        81 => match face {
            BlockFace::Top => (5, 4),
            BlockFace::Bottom => (7, 4),
            _ => (6, 4),
        },
        82 => (8, 4),
        83 => (9, 4),
        84 => match face {
            BlockFace::Top => (11, 4),
            BlockFace::Bottom => (4, 0),
            _ => (10, 4),
        },
        85 => (4, 0),
        86 => match face {
            BlockFace::Top | BlockFace::Bottom => (6, 6),
            BlockFace::North => (7, 7),
            _ => (6, 7),
        },
        87 => (7, 6),
        88 => (8, 6),
        89 => (9, 6),
        90 => (14, 0),
        91 => match face {
            BlockFace::Top | BlockFace::Bottom => (6, 6),
            BlockFace::North => (8, 7),
            _ => (6, 7),
        },
        92 => match face {
            BlockFace::Top => (9, 7),
            BlockFace::Bottom => (12, 7),
            _ => (10, 7),
        },
        93 => (3, 8),
        94 => (3, 9),
        96 => (4, 5),
        97 => (1, 0),
        98 => (6, 3),
        101 => (5, 5),
        102 => (1, 3),
        103 => match face {
            BlockFace::Top | BlockFace::Bottom => (9, 8),
            _ => (8, 8),
        },
        106 => (15, 8),
        107 => (4, 0),
        108 => (7, 0),
        109 => (6, 3),
        110 => match face {
            BlockFace::Top => (14, 4),
            BlockFace::Bottom => (2, 0),
            _ => (13, 4),
        },
        111 => (12, 4),
        112 => (0, 14),
        113 => (0, 14),
        114 => (0, 14),
        116 => match face {
            BlockFace::Top => (6, 10),
            BlockFace::Bottom => (7, 11),
            _ => (6, 11),
        },
        117 => (12, 9),
        118 => match face {
            BlockFace::Top => (10, 8),
            BlockFace::Bottom => (11, 9),
            _ => (10, 9),
        },
        120 => match face {
            BlockFace::Top => (14, 9),
            BlockFace::Bottom => (15, 10),
            _ => (15, 9),
        },
        121 => (15, 10),
        123 => (3, 13),
        124 => (4, 13),
        126 => match face {
            BlockFace::Top | BlockFace::Bottom => (4, 0),
            _ => (5, 0),
        },
        128 => match face {
            BlockFace::Top => (0, 11),
            BlockFace::Bottom => (0, 13),
            _ => (0, 12),
        },
        129 => (11, 10),
        130 => (5, 2),
        131 => (12, 10),
        133 => (9, 1),
        134 => (4, 0),
        135 => (4, 0),
        136 => (4, 0),
        139 => (0, 1),
        140 => (10, 11),
        143 => (4, 0),
        145 => match face {
            BlockFace::Top => (7, 14),
            _ => (7, 13),
        },
        153 => (15, 11),
        155 => match face {
            BlockFace::Top => (11, 13),
            BlockFace::Bottom => (11, 15),
            _ => (11, 14),
        },
        156 => match face {
            BlockFace::Top => (11, 13),
            BlockFace::Bottom => (11, 15),
            _ => (11, 14),
        },
        171 => (0, 4),
        REDSTONE_WIRE_BLOCK_ID | REDSTONE_WIRE_POWERED_BLOCK_ID => (4, 10),
        WATER_SOURCE_BLOCK_ID | WATER_FLOWING_BLOCK_ID => match face {
            BlockFace::Top | BlockFace::Bottom => (13, 12),
            _ => (14, 12),
        },
        LAVA_SOURCE_BLOCK_ID | LAVA_FLOWING_BLOCK_ID => match face {
            BlockFace::Top | BlockFace::Bottom => (13, 14),
            _ => (14, 14),
        },
        _ if block_id <= 255 => ((block_id % 16) as u8, (block_id / 16) as u8),
        _ => (1, 0),
    }
}

fn append_face(mesh: &mut TerrainMeshData, block_pos: BlockPos, block_id: u16, face_def: FaceDef) {
    let (tile_x, tile_y) = atlas_tile_for_block_face(block_id, face_def.face);
    append_face_with_tile(mesh, block_pos, block_id, face_def, tile_x, tile_y);
}

fn append_face_with_tile(
    mesh: &mut TerrainMeshData,
    block_pos: BlockPos,
    block_id: u16,
    face_def: FaceDef,
    tile_x: u8,
    tile_y: u8,
) {
    let base = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX - 4);
    let uv = atlas_uv(tile_x, tile_y);
    let color = vertex_tint_for_face(block_id, face_def.face);

    for (index, corner) in face_def.corners.iter().enumerate() {
        mesh.positions.push([
            block_pos.x as f32 + corner[0],
            block_pos.y as f32 + corner[1],
            block_pos.z as f32 + corner[2],
        ]);
        mesh.normals.push(face_def.normal);
        mesh.uvs.push(uv[face_def.uv_indices[index]]);
        mesh.colors.push(color);
    }

    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.face_render_layer.push(block_render_layer(block_id));
}

fn append_oriented_piston_block(
    world: &BlockWorld,
    mesh: &mut TerrainMeshData,
    block_pos: BlockPos,
    block_id: u16,
) {
    let facing = world.block_data(block_pos) & 0x7;

    for face in FACE_DEFS {
        let neighbor = BlockPos::new(
            block_pos.x + face.neighbor[0],
            block_pos.y + face.neighbor[1],
            block_pos.z + face.neighbor[2],
        );
        if is_face_occluding_block(world.block_id(neighbor)) {
            continue;
        }

        let (tile_x, tile_y) = piston_face_tile(block_id, facing, face.face);
        append_face_with_tile(mesh, block_pos, block_id, face, tile_x, tile_y);
    }
}

fn piston_face_tile(block_id: u16, facing: u8, face: BlockFace) -> (u8, u8) {
    let facing_face = block_face_from_facing_data(facing);
    let back_face = opposite_face(facing_face);

    if face == facing_face {
        if block_id == 29 { (10, 6) } else { (11, 6) }
    } else if face == back_face {
        (13, 6)
    } else {
        (12, 6)
    }
}

fn block_face_from_facing_data(facing: u8) -> BlockFace {
    match facing {
        0 => BlockFace::Bottom,
        1 => BlockFace::Top,
        2 => BlockFace::North,
        3 => BlockFace::South,
        4 => BlockFace::West,
        5 => BlockFace::East,
        _ => BlockFace::Top,
    }
}

fn opposite_face(face: BlockFace) -> BlockFace {
    match face {
        BlockFace::Top => BlockFace::Bottom,
        BlockFace::Bottom => BlockFace::Top,
        BlockFace::North => BlockFace::South,
        BlockFace::South => BlockFace::North,
        BlockFace::West => BlockFace::East,
        BlockFace::East => BlockFace::West,
    }
}

fn append_redstone_wire(
    world: &BlockWorld,
    mesh: &mut TerrainMeshData,
    block_pos: BlockPos,
    block_id: u16,
) {
    let y = block_pos.y as f32 + REDSTONE_WIRE_RENDER_OFFSET;
    let color = if block_id == REDSTONE_WIRE_POWERED_BLOCK_ID {
        REDSTONE_WIRE_LIT_TINT
    } else {
        REDSTONE_WIRE_UNLIT_TINT
    };

    let (w, e, n, s) = redstone_connections(world, block_pos);
    let mut x0 = block_pos.x as f32;
    let mut x1 = block_pos.x as f32 + 1.0;
    let mut z0 = block_pos.z as f32;
    let mut z1 = block_pos.z as f32 + 1.0;

    let pic = if (w || e) && !n && !s {
        1
    } else if (n || s) && !e && !w {
        2
    } else {
        0
    };

    if pic == 0 {
        if !w {
            x0 += REDSTONE_WIRE_CUT_DISTANCE;
        }
        if !e {
            x1 -= REDSTONE_WIRE_CUT_DISTANCE;
        }
        if !n {
            z0 += REDSTONE_WIRE_CUT_DISTANCE;
        }
        if !s {
            z1 -= REDSTONE_WIRE_CUT_DISTANCE;
        }
    }

    let base_uv = if pic == 0 {
        let mut u0 = 0.0;
        let mut v0 = 0.0;
        let mut u1 = 16.0;
        let mut v1 = 16.0;
        let cut = REDSTONE_WIRE_CUT_DISTANCE * 16.0;
        if !w {
            u0 += cut;
        }
        if !e {
            u1 -= cut;
        }
        if !n {
            v0 += cut;
        }
        if !s {
            v1 -= cut;
        }
        atlas_uv_region(4, 10, u0, v0, u1, v1)
    } else if pic == 1 {
        atlas_uv(5, 10)
    } else {
        let uv = atlas_uv(5, 10);
        [uv[1], uv[0], uv[3], uv[2]]
    };

    append_redstone_wire_quad(mesh, x0, x1, z0, z1, y, base_uv, color);

    let overlay_uv = if pic == 0 {
        let mut u0 = 0.0;
        let mut v0 = 0.0;
        let mut u1 = 16.0;
        let mut v1 = 16.0;
        let cut = REDSTONE_WIRE_CUT_DISTANCE * 16.0;
        if !w {
            u0 += cut;
        }
        if !e {
            u1 -= cut;
        }
        if !n {
            v0 += cut;
        }
        if !s {
            v1 -= cut;
        }
        atlas_uv_region(4, 11, u0, v0, u1, v1)
    } else if pic == 1 {
        atlas_uv(5, 11)
    } else {
        let uv = atlas_uv(5, 11);
        [uv[1], uv[0], uv[3], uv[2]]
    };
    append_redstone_wire_quad(
        mesh,
        x0,
        x1,
        z0,
        z1,
        y + 0.0001,
        overlay_uv,
        [1.0, 1.0, 1.0, 1.0],
    );
}

fn redstone_connections(world: &BlockWorld, block_pos: BlockPos) -> (bool, bool, bool, bool) {
    (
        redstone_connects_in_direction(world, block_pos, -1, 0),
        redstone_connects_in_direction(world, block_pos, 1, 0),
        redstone_connects_in_direction(world, block_pos, 0, -1),
        redstone_connects_in_direction(world, block_pos, 0, 1),
    )
}

fn redstone_connects_in_direction(
    world: &BlockWorld,
    block_pos: BlockPos,
    dx: i32,
    dz: i32,
) -> bool {
    let neighbor = BlockPos::new(block_pos.x + dx, block_pos.y, block_pos.z + dz);
    if redstone_connection_target(world.block_id(neighbor)) {
        return true;
    }

    let block_above = BlockPos::new(block_pos.x, block_pos.y + 1, block_pos.z);
    if !is_block_motion_solid(world.block_id(neighbor)) {
        let below = BlockPos::new(neighbor.x, neighbor.y - 1, neighbor.z);
        if redstone_connection_target(world.block_id(below)) {
            return true;
        }
    } else if !is_block_motion_solid(world.block_id(block_above)) {
        let above = BlockPos::new(neighbor.x, neighbor.y + 1, neighbor.z);
        if redstone_connection_target(world.block_id(above)) {
            return true;
        }
    }

    false
}

fn redstone_connection_target(block_id: u16) -> bool {
    block_id == REDSTONE_WIRE_BLOCK_ID
        || block_id == REDSTONE_WIRE_POWERED_BLOCK_ID
        || is_redstone_block(block_id)
}

fn append_redstone_wire_quad(
    mesh: &mut TerrainMeshData,
    x0: f32,
    x1: f32,
    z0: f32,
    z1: f32,
    y: f32,
    uv: [[f32; 2]; 4],
    color: [f32; 4],
) {
    let base = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX - 4);

    let corners = [[x0, y, z0], [x0, y, z1], [x1, y, z1], [x1, y, z0]];

    for index in 0..4 {
        mesh.positions.push(corners[index]);
        mesh.normals.push([0.0, 1.0, 0.0]);
        mesh.uvs.push(uv[index]);
        mesh.colors.push(color);
    }

    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.face_render_layer
        .push(block_render_layer(REDSTONE_WIRE_BLOCK_ID));
}

fn atlas_uv_region(
    tile_x: u8,
    tile_y: u8,
    u0_px: f32,
    v0_px: f32,
    u1_px: f32,
    v1_px: f32,
) -> [[f32; 2]; 4] {
    let full = atlas_uv(tile_x, tile_y);
    let tile_u0 = full[0][0];
    let tile_u1 = full[1][0];
    let tile_v0 = full[2][1];
    let tile_v1 = full[0][1];

    let u0 = tile_u0 + (tile_u1 - tile_u0) * (u0_px / 16.0);
    let u1 = tile_u0 + (tile_u1 - tile_u0) * (u1_px / 16.0);
    let v0 = tile_v0 + (tile_v1 - tile_v0) * (v0_px / 16.0);
    let v1 = tile_v0 + (tile_v1 - tile_v0) * (v1_px / 16.0);

    [[u0, v1], [u0, v0], [u1, v0], [u1, v1]]
}

fn is_torch_block(block_id: u16) -> bool {
    matches!(block_id, 50 | 75 | 76)
}

fn append_torch(
    world: &BlockWorld,
    mesh: &mut TerrainMeshData,
    block_pos: BlockPos,
    block_id: u16,
) {
    let data = world.block_data(block_pos) & 0x7;
    let (x0, y0, z0, x1, y1, z1) = match data {
        1 => (0.0, 0.2, 0.35, 0.3, 0.8, 0.65),
        2 => (0.7, 0.2, 0.35, 1.0, 0.8, 0.65),
        3 => (0.35, 0.2, 0.0, 0.65, 0.8, 0.3),
        4 => (0.35, 0.2, 0.7, 0.65, 0.8, 1.0),
        _ => (0.4, 0.0, 0.4, 0.6, 0.6, 0.6),
    };

    append_axis_aligned_box(
        mesh,
        block_pos,
        block_id,
        [x0, y0, z0, x1, y1, z1],
        [1.0, 1.0, 1.0, 1.0],
        block_render_layer(block_id),
    );
}

fn append_lever(
    world: &BlockWorld,
    mesh: &mut TerrainMeshData,
    block_pos: BlockPos,
    block_id: u16,
) {
    let dir = world.block_data(block_pos) & 0x7;
    let bounds = match dir {
        1 => [0.0, 0.2, 0.3125, 0.375, 0.8, 0.6875],
        2 => [0.625, 0.2, 0.3125, 1.0, 0.8, 0.6875],
        3 => [0.3125, 0.2, 0.0, 0.6875, 0.8, 0.375],
        4 => [0.3125, 0.2, 0.625, 0.6875, 0.8, 1.0],
        5 | 6 => [0.25, 0.0, 0.25, 0.75, 0.6, 0.75],
        0 | 7 => [0.25, 0.4, 0.25, 0.75, 1.0, 0.75],
        _ => [0.25, 0.0, 0.25, 0.75, 0.6, 0.75],
    };

    append_axis_aligned_box(
        mesh,
        block_pos,
        block_id,
        bounds,
        [1.0, 1.0, 1.0, 1.0],
        block_render_layer(block_id),
    );
}

fn append_axis_aligned_box(
    mesh: &mut TerrainMeshData,
    block_pos: BlockPos,
    block_id: u16,
    bounds: [f32; 6],
    color: [f32; 4],
    render_layer: u8,
) {
    let [x0, y0, z0, x1, y1, z1] = bounds;

    append_box_face(
        mesh,
        [
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z0,
            ],
        ],
        [0.0, 1.0, 0.0],
        atlas_uv(
            atlas_tile_for_block_face(block_id, BlockFace::Top).0,
            atlas_tile_for_block_face(block_id, BlockFace::Top).1,
        ),
        color,
        render_layer,
    );
    append_box_face(
        mesh,
        [
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z1,
            ],
        ],
        [0.0, -1.0, 0.0],
        atlas_uv(
            atlas_tile_for_block_face(block_id, BlockFace::Bottom).0,
            atlas_tile_for_block_face(block_id, BlockFace::Bottom).1,
        ),
        color,
        render_layer,
    );
    append_box_face(
        mesh,
        [
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z0,
            ],
        ],
        [0.0, 0.0, -1.0],
        atlas_uv(
            atlas_tile_for_block_face(block_id, BlockFace::North).0,
            atlas_tile_for_block_face(block_id, BlockFace::North).1,
        ),
        color,
        render_layer,
    );
    append_box_face(
        mesh,
        [
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z1,
            ],
        ],
        [0.0, 0.0, 1.0],
        atlas_uv(
            atlas_tile_for_block_face(block_id, BlockFace::South).0,
            atlas_tile_for_block_face(block_id, BlockFace::South).1,
        ),
        color,
        render_layer,
    );
    append_box_face(
        mesh,
        [
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x0,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z1,
            ],
        ],
        [-1.0, 0.0, 0.0],
        atlas_uv(
            atlas_tile_for_block_face(block_id, BlockFace::West).0,
            atlas_tile_for_block_face(block_id, BlockFace::West).1,
        ),
        color,
        render_layer,
    );
    append_box_face(
        mesh,
        [
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z1,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y1,
                block_pos.z as f32 + z0,
            ],
            [
                block_pos.x as f32 + x1,
                block_pos.y as f32 + y0,
                block_pos.z as f32 + z0,
            ],
        ],
        [1.0, 0.0, 0.0],
        atlas_uv(
            atlas_tile_for_block_face(block_id, BlockFace::East).0,
            atlas_tile_for_block_face(block_id, BlockFace::East).1,
        ),
        color,
        render_layer,
    );
}

fn append_box_face(
    mesh: &mut TerrainMeshData,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    uv: [[f32; 2]; 4],
    color: [f32; 4],
    render_layer: u8,
) {
    let base = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX - 4);
    for index in 0..4 {
        mesh.positions.push(corners[index]);
        mesh.normals.push(normal);
        mesh.uvs.push(uv[index]);
        mesh.colors.push(color);
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.face_render_layer.push(render_layer);
}

fn flat_top_block_tile(block_id: u16) -> Option<(u8, u8)> {
    let tile = match block_id {
        27 => (3, 10),
        28 => (3, 12),
        66 => (0, 8),
        70 => (1, 0),
        72 => (4, 0),
        93 => (3, 8),
        94 => (3, 9),
        111 => (12, 4),
        131 => (12, 10),
        _ => return None,
    };

    Some(tile)
}

fn cross_plane_block_tile(block_id: u16) -> Option<(u8, u8)> {
    let tile = match block_id {
        31 => (7, 2),
        32 => (7, 3),
        37 => (13, 0),
        38 => (12, 0),
        39 => (13, 1),
        40 => (12, 1),
        59 => (15, 5),
        83 => (9, 4),
        106 => (15, 8),
        _ => return None,
    };

    Some(tile)
}

fn append_flat_tile(
    mesh: &mut TerrainMeshData,
    block_pos: BlockPos,
    tile_x: u8,
    tile_y: u8,
    y: f32,
    render_layer: u8,
) {
    let uv = atlas_uv(tile_x, tile_y);
    let world_y = block_pos.y as f32 + y;
    let base = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX - 4);
    let color = [1.0, 1.0, 1.0, 1.0];

    let corners = [
        [block_pos.x as f32, world_y, block_pos.z as f32],
        [block_pos.x as f32, world_y, block_pos.z as f32 + 1.0],
        [block_pos.x as f32 + 1.0, world_y, block_pos.z as f32 + 1.0],
        [block_pos.x as f32 + 1.0, world_y, block_pos.z as f32],
    ];

    for index in 0..4 {
        mesh.positions.push(corners[index]);
        mesh.normals.push([0.0, 1.0, 0.0]);
        mesh.uvs.push(uv[index]);
        mesh.colors.push(color);
    }

    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.face_render_layer.push(render_layer);
}

fn append_cross_plane_block(
    mesh: &mut TerrainMeshData,
    block_pos: BlockPos,
    tile_x: u8,
    tile_y: u8,
    render_layer: u8,
) {
    let uv = atlas_uv(tile_x, tile_y);
    let color = [1.0, 1.0, 1.0, 1.0];

    append_cross_plane_quad(
        mesh,
        [
            [
                block_pos.x as f32 + 0.0,
                block_pos.y as f32 + 0.0,
                block_pos.z as f32 + 0.0,
            ],
            [
                block_pos.x as f32 + 0.0,
                block_pos.y as f32 + 1.0,
                block_pos.z as f32 + 0.0,
            ],
            [
                block_pos.x as f32 + 1.0,
                block_pos.y as f32 + 1.0,
                block_pos.z as f32 + 1.0,
            ],
            [
                block_pos.x as f32 + 1.0,
                block_pos.y as f32 + 0.0,
                block_pos.z as f32 + 1.0,
            ],
        ],
        uv,
        color,
        render_layer,
    );

    append_cross_plane_quad(
        mesh,
        [
            [
                block_pos.x as f32 + 1.0,
                block_pos.y as f32 + 0.0,
                block_pos.z as f32 + 0.0,
            ],
            [
                block_pos.x as f32 + 1.0,
                block_pos.y as f32 + 1.0,
                block_pos.z as f32 + 0.0,
            ],
            [
                block_pos.x as f32 + 0.0,
                block_pos.y as f32 + 1.0,
                block_pos.z as f32 + 1.0,
            ],
            [
                block_pos.x as f32 + 0.0,
                block_pos.y as f32 + 0.0,
                block_pos.z as f32 + 1.0,
            ],
        ],
        uv,
        color,
        render_layer,
    );
}

fn append_cross_plane_quad(
    mesh: &mut TerrainMeshData,
    corners: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    color: [f32; 4],
    render_layer: u8,
) {
    let base = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX - 8);

    for index in 0..4 {
        mesh.positions.push(corners[index]);
        mesh.normals.push([0.0, 0.0, 1.0]);
        mesh.uvs.push(uv[index]);
        mesh.colors.push(color);
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.face_render_layer.push(render_layer);

    let back_base = base + 4;
    for index in 0..4 {
        mesh.positions.push(corners[index]);
        mesh.normals.push([0.0, 0.0, -1.0]);
        mesh.uvs.push(uv[index]);
        mesh.colors.push(color);
    }
    mesh.indices.extend_from_slice(&[
        back_base,
        back_base + 3,
        back_base + 2,
        back_base,
        back_base + 2,
        back_base + 1,
    ]);
    mesh.face_render_layer.push(render_layer);
}

fn append_fluid_block(
    mesh_world: &BlockWorld,
    mesh: &mut TerrainMeshData,
    pos: BlockPos,
    block_id: u16,
) {
    let Some(kind) = fluid_kind_for_block(block_id) else {
        return;
    };

    let mut render_north = should_render_fluid_face(
        mesh_world,
        BlockPos::new(pos.x, pos.y, pos.z - 1),
        kind,
        BlockFace::North,
    );
    let mut render_south = should_render_fluid_face(
        mesh_world,
        BlockPos::new(pos.x, pos.y, pos.z + 1),
        kind,
        BlockFace::South,
    );
    let mut render_west = should_render_fluid_face(
        mesh_world,
        BlockPos::new(pos.x - 1, pos.y, pos.z),
        kind,
        BlockFace::West,
    );
    let mut render_east = should_render_fluid_face(
        mesh_world,
        BlockPos::new(pos.x + 1, pos.y, pos.z),
        kind,
        BlockFace::East,
    );
    let render_up = should_render_fluid_face(
        mesh_world,
        BlockPos::new(pos.x, pos.y + 1, pos.z),
        kind,
        BlockFace::Top,
    );
    let render_down = should_render_fluid_face(
        mesh_world,
        BlockPos::new(pos.x, pos.y - 1, pos.z),
        kind,
        BlockFace::Bottom,
    );

    if !(render_up || render_down || render_north || render_south || render_west || render_east) {
        return;
    }

    let mut h0 = fluid_corner_height(mesh_world, pos.x, pos.y, pos.z, kind);
    let mut h1 = fluid_corner_height(mesh_world, pos.x, pos.y, pos.z + 1, kind);
    let mut h2 = fluid_corner_height(mesh_world, pos.x + 1, pos.y, pos.z + 1, kind);
    let mut h3 = fluid_corner_height(mesh_world, pos.x + 1, pos.y, pos.z, kind);

    let max_h = h0.max(h1).max(h2).max(h3);
    if max_h <= (15.0 / 16.0) {
        if mesh_world.block_id(BlockPos::new(pos.x, pos.y, pos.z - 1)) == FARMLAND_BLOCK_ID {
            render_north = false;
        }
        if mesh_world.block_id(BlockPos::new(pos.x, pos.y, pos.z + 1)) == FARMLAND_BLOCK_ID {
            render_south = false;
        }
        if mesh_world.block_id(BlockPos::new(pos.x - 1, pos.y, pos.z)) == FARMLAND_BLOCK_ID {
            render_west = false;
        }
        if mesh_world.block_id(BlockPos::new(pos.x + 1, pos.y, pos.z)) == FARMLAND_BLOCK_ID {
            render_east = false;
        }
    }

    let color = fluid_vertex_tint(kind);
    let render_layer = fluid_render_layer(kind);

    if render_up {
        h0 -= FLUID_FACE_OFFSET;
        h1 -= FLUID_FACE_OFFSET;
        h2 -= FLUID_FACE_OFFSET;
        h3 -= FLUID_FACE_OFFSET;

        let flow = fluid_flow_xz(mesh_world, pos, kind);
        let top_uv = if flow.0.abs() <= f32::EPSILON && flow.1.abs() <= f32::EPSILON {
            fluid_top_still_uv(kind)
        } else {
            fluid_top_flow_uv(kind, flow.1.atan2(flow.0) - std::f32::consts::FRAC_PI_2)
        };

        append_quad(
            mesh,
            [
                [pos.x as f32, pos.y as f32 + h0, pos.z as f32],
                [pos.x as f32, pos.y as f32 + h1, pos.z as f32 + 1.0],
                [pos.x as f32 + 1.0, pos.y as f32 + h2, pos.z as f32 + 1.0],
                [pos.x as f32 + 1.0, pos.y as f32 + h3, pos.z as f32],
            ],
            [0.0, 1.0, 0.0],
            top_uv,
            color,
            render_layer,
        );
    }

    if render_down {
        let down_uv = fluid_top_still_uv(kind);
        append_quad(
            mesh,
            [
                [
                    pos.x as f32,
                    pos.y as f32 + FLUID_FACE_OFFSET,
                    pos.z as f32 + 1.0,
                ],
                [
                    pos.x as f32 + 1.0,
                    pos.y as f32 + FLUID_FACE_OFFSET,
                    pos.z as f32 + 1.0,
                ],
                [
                    pos.x as f32 + 1.0,
                    pos.y as f32 + FLUID_FACE_OFFSET,
                    pos.z as f32,
                ],
                [pos.x as f32, pos.y as f32 + FLUID_FACE_OFFSET, pos.z as f32],
            ],
            [0.0, -1.0, 0.0],
            down_uv,
            color,
            render_layer,
        );
    }

    if render_north {
        append_fluid_side(
            mesh,
            kind,
            [
                [
                    pos.x as f32,
                    pos.y as f32 + h0,
                    pos.z as f32 + FLUID_FACE_OFFSET,
                ],
                [
                    pos.x as f32 + 1.0,
                    pos.y as f32 + h3,
                    pos.z as f32 + FLUID_FACE_OFFSET,
                ],
                [
                    pos.x as f32 + 1.0,
                    pos.y as f32,
                    pos.z as f32 + FLUID_FACE_OFFSET,
                ],
                [pos.x as f32, pos.y as f32, pos.z as f32 + FLUID_FACE_OFFSET],
            ],
            [0.0, 0.0, -1.0],
            h0,
            h3,
            color,
            render_layer,
        );
    }

    if render_south {
        append_fluid_side(
            mesh,
            kind,
            [
                [
                    pos.x as f32 + 1.0,
                    pos.y as f32 + h2,
                    pos.z as f32 + 1.0 - FLUID_FACE_OFFSET,
                ],
                [
                    pos.x as f32,
                    pos.y as f32 + h1,
                    pos.z as f32 + 1.0 - FLUID_FACE_OFFSET,
                ],
                [
                    pos.x as f32,
                    pos.y as f32,
                    pos.z as f32 + 1.0 - FLUID_FACE_OFFSET,
                ],
                [
                    pos.x as f32 + 1.0,
                    pos.y as f32,
                    pos.z as f32 + 1.0 - FLUID_FACE_OFFSET,
                ],
            ],
            [0.0, 0.0, 1.0],
            h2,
            h1,
            color,
            render_layer,
        );
    }

    if render_west {
        append_fluid_side(
            mesh,
            kind,
            [
                [
                    pos.x as f32 + FLUID_FACE_OFFSET,
                    pos.y as f32 + h1,
                    pos.z as f32 + 1.0,
                ],
                [
                    pos.x as f32 + FLUID_FACE_OFFSET,
                    pos.y as f32 + h0,
                    pos.z as f32,
                ],
                [pos.x as f32 + FLUID_FACE_OFFSET, pos.y as f32, pos.z as f32],
                [
                    pos.x as f32 + FLUID_FACE_OFFSET,
                    pos.y as f32,
                    pos.z as f32 + 1.0,
                ],
            ],
            [-1.0, 0.0, 0.0],
            h1,
            h0,
            color,
            render_layer,
        );
    }

    if render_east {
        append_fluid_side(
            mesh,
            kind,
            [
                [
                    pos.x as f32 + 1.0 - FLUID_FACE_OFFSET,
                    pos.y as f32 + h3,
                    pos.z as f32,
                ],
                [
                    pos.x as f32 + 1.0 - FLUID_FACE_OFFSET,
                    pos.y as f32 + h2,
                    pos.z as f32 + 1.0,
                ],
                [
                    pos.x as f32 + 1.0 - FLUID_FACE_OFFSET,
                    pos.y as f32,
                    pos.z as f32 + 1.0,
                ],
                [
                    pos.x as f32 + 1.0 - FLUID_FACE_OFFSET,
                    pos.y as f32,
                    pos.z as f32,
                ],
            ],
            [1.0, 0.0, 0.0],
            h3,
            h2,
            color,
            render_layer,
        );
    }
}

fn append_fluid_side(
    mesh: &mut TerrainMeshData,
    kind: FluidKind,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    h0: f32,
    h1: f32,
    color: [f32; 4],
    render_layer: u8,
) {
    let (u0, v0, u1, v1) = fluid_flow_uv_rect(kind);
    let u_mid = u0 + (u1 - u0) * 0.5;
    let uv = [
        [u0, v0 + (v1 - v0) * ((1.0 - h0) * 0.5)],
        [u_mid, v0 + (v1 - v0) * ((1.0 - h1) * 0.5)],
        [u_mid, v0 + (v1 - v0) * 0.5],
        [u0, v0 + (v1 - v0) * 0.5],
    ];
    append_quad(mesh, corners, normal, uv, color, render_layer);
}

fn append_quad(
    mesh: &mut TerrainMeshData,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    uv: [[f32; 2]; 4],
    color: [f32; 4],
    render_layer: u8,
) {
    let base = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX - 4);
    for index in 0..4 {
        mesh.positions.push(corners[index]);
        mesh.normals.push(normal);
        mesh.uvs.push(uv[index]);
        mesh.colors.push(color);
    }

    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.face_render_layer.push(render_layer);
}

fn fluid_render_layer(kind: FluidKind) -> u8 {
    match kind {
        // `LiquidTile::getRenderLayer`: water is translucent layer 1, lava remains layer 0.
        FluidKind::Water => 1,
        FluidKind::Lava => 0,
    }
}

fn block_render_layer(block_id: u16) -> u8 {
    match block_id {
        WATER_SOURCE_BLOCK_ID | WATER_FLOWING_BLOCK_ID | 79 | 90 | 95 | 132 | 160 => 1,
        _ => 0,
    }
}

fn should_render_fluid_face(
    world: &BlockWorld,
    neighbor: BlockPos,
    kind: FluidKind,
    face: BlockFace,
) -> bool {
    let neighbor_id = world.block_id(neighbor);
    if fluid_kind_for_block(neighbor_id) == Some(kind) {
        return false;
    }

    if face == BlockFace::Top {
        return true;
    }

    if neighbor_id == 79 {
        return false;
    }

    !is_block_motion_solid(neighbor_id)
}

fn fluid_corner_height(world: &BlockWorld, x: i32, y: i32, z: i32, kind: FluidKind) -> f32 {
    let mut count = 0;
    let mut h = 0.0;

    for i in 0..4 {
        let xx = x - (i & 1);
        let zz = z - ((i >> 1) & 1);

        let above = BlockPos::new(xx, y + 1, zz);
        if fluid_kind_for_block(world.block_id(above)) == Some(kind) {
            return 1.0;
        }

        let here = BlockPos::new(xx, y, zz);
        if fluid_kind_for_block(world.block_id(here)) == Some(kind) {
            let data = world.block_data(here);
            let height = fluid_height_from_data(data);
            if data >= 8 || data == 0 {
                h += height * 10.0;
                count += 10;
            }
            h += height;
            count += 1;
        } else if !is_block_motion_solid(world.block_id(here)) {
            h += 1.0;
            count += 1;
        }
    }

    if count == 0 {
        1.0
    } else {
        1.0 - h / count as f32
    }
}

fn fluid_height_from_data(data: u8) -> f32 {
    let rendered = if data >= 8 { 0 } else { i32::from(data) };
    (rendered as f32 + 1.0) / 9.0
}

fn fluid_flow_xz(world: &BlockWorld, block: BlockPos, kind: FluidKind) -> (f32, f32) {
    let mid = rendered_depth(world, block, kind);
    if mid < 0 {
        return (0.0, 0.0);
    }

    let mut flow_x = 0.0f32;
    let mut flow_z = 0.0f32;

    let neighbors = [
        BlockPos::new(block.x - 1, block.y, block.z),
        BlockPos::new(block.x, block.y, block.z - 1),
        BlockPos::new(block.x + 1, block.y, block.z),
        BlockPos::new(block.x, block.y, block.z + 1),
    ];

    for neighbor in neighbors {
        let mut neighbor_depth = rendered_depth(world, neighbor, kind);
        if neighbor_depth < 0 {
            if !is_block_motion_solid(world.block_id(neighbor)) {
                let below = BlockPos::new(neighbor.x, neighbor.y - 1, neighbor.z);
                neighbor_depth = rendered_depth(world, below, kind);
                if neighbor_depth >= 0 {
                    let dir = (neighbor_depth - (mid - 8)) as f32;
                    flow_x += (neighbor.x - block.x) as f32 * dir;
                    flow_z += (neighbor.z - block.z) as f32 * dir;
                }
            }
        } else {
            let dir = (neighbor_depth - mid) as f32;
            flow_x += (neighbor.x - block.x) as f32 * dir;
            flow_z += (neighbor.z - block.z) as f32 * dir;
        }
    }

    let len = (flow_x * flow_x + flow_z * flow_z).sqrt();
    if len <= f32::EPSILON {
        (0.0, 0.0)
    } else {
        (flow_x / len, flow_z / len)
    }
}

fn rendered_depth(world: &BlockWorld, block: BlockPos, kind: FluidKind) -> i32 {
    if fluid_kind_for_block(world.block_id(block)) != Some(kind) {
        return -1;
    }

    let mut depth = i32::from(world.block_data(block));
    if depth >= 8 {
        depth = 0;
    }
    depth
}

fn is_face_occluding_block(block_id: u16) -> bool {
    if block_id == 0 || is_fluid_block(block_id) {
        return false;
    }

    !matches!(
        block_id,
        REDSTONE_WIRE_BLOCK_ID
            | REDSTONE_WIRE_POWERED_BLOCK_ID
            | 27
            | 28
            | 31
            | 32
            | 37
            | 38
            | 39
            | 40
            | 50
            | 59
            | 63
            | 64
            | 65
            | 66
            | 68
            | 69
            | 70
            | 71
            | 72
            | 75
            | 76
            | 77
            | 78
            | 83
            | 90
            | 93
            | 94
            | 96
            | 101
            | 102
            | 106
            | 111
            | 131
            | 140
            | 143
            | 171
    )
}

fn is_block_motion_solid(block_id: u16) -> bool {
    if block_id == 0 || is_fluid_block(block_id) {
        return false;
    }

    !matches!(
        block_id,
        REDSTONE_WIRE_BLOCK_ID
            | REDSTONE_WIRE_POWERED_BLOCK_ID
            | 27
            | 28
            | 31
            | 32
            | 37
            | 38
            | 39
            | 40
            | 50
            | 59
            | 63
            | 64
            | 65
            | 66
            | 68
            | 69
            | 70
            | 71
            | 72
            | 75
            | 76
            | 77
            | 78
            | 83
            | 90
            | 93
            | 94
            | 96
            | 101
            | 102
            | 106
            | 111
            | 131
            | 140
            | 143
            | 171
    )
}

fn fluid_vertex_tint(kind: FluidKind) -> [f32; 4] {
    match kind {
        FluidKind::Water => WATER_TINT,
        FluidKind::Lava => LAVA_TINT,
    }
}

fn fluid_top_still_uv(kind: FluidKind) -> [[f32; 2]; 4] {
    match kind {
        FluidKind::Water => atlas_uv(13, 12),
        FluidKind::Lava => atlas_uv(13, 14),
    }
}

fn fluid_flow_uv_rect(kind: FluidKind) -> (f32, f32, f32, f32) {
    match kind {
        FluidKind::Water => atlas_uv_rect(14, 12, 2, 2),
        FluidKind::Lava => atlas_uv_rect(14, 14, 2, 2),
    }
}

fn fluid_top_flow_uv(kind: FluidKind, angle: f32) -> [[f32; 2]; 4] {
    let (u0, v0, u1, v1) = fluid_flow_uv_rect(kind);
    let s = angle.sin() * 0.25;
    let c = angle.cos() * 0.25;

    let to_u = |t: f32| u0 + (u1 - u0) * t;
    let to_v = |t: f32| v0 + (v1 - v0) * t;

    [
        [to_u(0.5 + (-c - s)), to_v(0.5 + (-c + s))],
        [to_u(0.5 + (-c + s)), to_v(0.5 + (c + s))],
        [to_u(0.5 + (c + s)), to_v(0.5 + (c - s))],
        [to_u(0.5 + (c - s)), to_v(0.5 + (-c - s))],
    ]
}

fn vertex_tint_for_face(block_id: u16, face: BlockFace) -> [f32; 4] {
    if block_id == 2 && face == BlockFace::Top {
        GRASS_TOP_TINT
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}

fn atlas_uv(tile_x: u8, tile_y: u8) -> [[f32; 2]; 4] {
    let tile = 1.0 / f32::from(TERRAIN_ATLAS_TILES);
    let u0 = f32::from(tile_x) * tile + ATLAS_UV_INSET;
    let v0 = f32::from(tile_y) * tile + ATLAS_UV_INSET;
    let u1 = (f32::from(tile_x) + 1.0) * tile - ATLAS_UV_INSET;
    let v1 = (f32::from(tile_y) + 1.0) * tile - ATLAS_UV_INSET;

    [[u0, v1], [u1, v1], [u1, v0], [u0, v0]]
}

fn atlas_uv_rect(
    tile_x: u8,
    tile_y: u8,
    span_x_tiles: u8,
    span_y_tiles: u8,
) -> (f32, f32, f32, f32) {
    let tile = 1.0 / f32::from(TERRAIN_ATLAS_TILES);
    let u0 = f32::from(tile_x) * tile + ATLAS_UV_INSET;
    let v0 = f32::from(tile_y) * tile + ATLAS_UV_INSET;
    let u1 = (f32::from(tile_x) + f32::from(span_x_tiles)) * tile - ATLAS_UV_INSET;
    let v1 = (f32::from(tile_y) + f32::from(span_y_tiles)) * tile - ATLAS_UV_INSET;

    (u0, v0, u1, v1)
}

#[derive(Clone, Copy)]
struct FaceDef {
    face: BlockFace,
    normal: [f32; 3],
    neighbor: [i32; 3],
    corners: [[f32; 3]; 4],
    uv_indices: [usize; 4],
}

const FACE_DEFS: [FaceDef; 6] = [
    FaceDef {
        face: BlockFace::Top,
        normal: [0.0, 1.0, 0.0],
        neighbor: [0, 1, 0],
        corners: [
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        uv_indices: [0, 1, 2, 3],
    },
    FaceDef {
        face: BlockFace::Bottom,
        normal: [0.0, -1.0, 0.0],
        neighbor: [0, -1, 0],
        corners: [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        uv_indices: [0, 1, 2, 3],
    },
    FaceDef {
        face: BlockFace::North,
        normal: [0.0, 0.0, -1.0],
        neighbor: [0, 0, -1],
        corners: [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        uv_indices: [0, 3, 2, 1],
    },
    FaceDef {
        face: BlockFace::South,
        normal: [0.0, 0.0, 1.0],
        neighbor: [0, 0, 1],
        corners: [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
        uv_indices: [0, 3, 2, 1],
    },
    FaceDef {
        face: BlockFace::West,
        normal: [-1.0, 0.0, 0.0],
        neighbor: [-1, 0, 0],
        corners: [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        uv_indices: [0, 3, 2, 1],
    },
    FaceDef {
        face: BlockFace::East,
        normal: [1.0, 0.0, 0.0],
        neighbor: [1, 0, 0],
        corners: [
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
        uv_indices: [0, 3, 2, 1],
    },
];

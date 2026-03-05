use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use crate::save::nbt::{
    NbtCompound, NbtError, NbtList, NbtRoot, NbtTag, TagType, read_root_from_bytes,
    write_root_to_bytes,
};
use crate::save::world_io::{WorldIoError, load_chunk_payload, save_chunk_payload};

const BLOCKS_PER_CHUNK: i32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn from_block(block: BlockPos) -> Self {
        Self {
            x: block.x.div_euclid(BLOCKS_PER_CHUNK),
            z: block.z.div_euclid(BLOCKS_PER_CHUNK),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LocalBlockPos {
    x: u8,
    y: i32,
    z: u8,
}

impl LocalBlockPos {
    fn from_block(block: BlockPos) -> Self {
        Self {
            x: u8::try_from(block.x.rem_euclid(BLOCKS_PER_CHUNK))
                .expect("local x coordinate should fit u8"),
            y: block.y,
            z: u8::try_from(block.z.rem_euclid(BLOCKS_PER_CHUNK))
                .expect("local z coordinate should fit u8"),
        }
    }

    fn to_world(self, chunk: ChunkPos) -> BlockPos {
        BlockPos {
            x: chunk.x * BLOCKS_PER_CHUNK + i32::from(self.x),
            y: self.y,
            z: chunk.z * BLOCKS_PER_CHUNK + i32::from(self.z),
        }
    }

    fn from_nbt(x: i8, y: i32, z: i8) -> Result<Self, BlockWorldError> {
        if !(0..=15).contains(&x) {
            return Err(BlockWorldError::InvalidLocalCoordinate {
                axis: 'x',
                value: i32::from(x),
            });
        }

        if !(0..=15).contains(&z) {
            return Err(BlockWorldError::InvalidLocalCoordinate {
                axis: 'z',
                value: i32::from(z),
            });
        }

        Ok(Self {
            x: u8::try_from(x).expect("x range checked"),
            y,
            z: u8::try_from(z).expect("z range checked"),
        })
    }
}

type ChunkBlocks = HashMap<LocalBlockPos, u16>;
type ChunkBlockData = HashMap<LocalBlockPos, u8>;

#[derive(Debug)]
pub enum BlockWorldError {
    Nbt(NbtError),
    WorldIo(WorldIoError),
    MissingField(&'static str),
    InvalidFieldType {
        field: &'static str,
        expected: TagType,
    },
    InvalidListEntryType,
    InvalidLocalCoordinate {
        axis: char,
        value: i32,
    },
    InvalidBlockId(i32),
    ChunkMismatch {
        expected: ChunkPos,
        found: ChunkPos,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkLoadOutcome {
    LoadedFromStorage,
    GeneratedFallback,
}

impl fmt::Display for BlockWorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nbt(error) => write!(f, "nbt error: {error}"),
            Self::WorldIo(error) => write!(f, "world io error: {error}"),
            Self::MissingField(field) => write!(f, "missing required field: {field}"),
            Self::InvalidFieldType { field, expected } => {
                write!(f, "invalid field type for {field}, expected {expected:?}")
            }
            Self::InvalidListEntryType => {
                write!(f, "blocks list must contain only compound entries")
            }
            Self::InvalidLocalCoordinate { axis, value } => {
                write!(f, "invalid local {axis} coordinate: {value}")
            }
            Self::InvalidBlockId(value) => write!(f, "invalid block id: {value}"),
            Self::ChunkMismatch { expected, found } => write!(
                f,
                "chunk payload mismatch, expected ({}, {}), found ({}, {})",
                expected.x, expected.z, found.x, found.z
            ),
        }
    }
}

impl std::error::Error for BlockWorldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Nbt(error) => Some(error),
            Self::WorldIo(error) => Some(error),
            _ => None,
        }
    }
}

impl From<NbtError> for BlockWorldError {
    fn from(error: NbtError) -> Self {
        Self::Nbt(error)
    }
}

impl From<WorldIoError> for BlockWorldError {
    fn from(error: WorldIoError) -> Self {
        Self::WorldIo(error)
    }
}

#[derive(Debug, Default)]
pub struct BlockWorld {
    chunks: HashMap<ChunkPos, ChunkBlocks>,
    block_data: HashMap<ChunkPos, ChunkBlockData>,
}

impl BlockWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn touched_chunks(&self) -> Vec<ChunkPos> {
        let mut chunks: Vec<_> = self.chunks.keys().copied().collect();
        chunks.sort();
        chunks
    }

    pub fn all_blocks(&self) -> Vec<(BlockPos, u16)> {
        let mut blocks = Vec::new();

        for (chunk_pos, chunk_blocks) in &self.chunks {
            for (local_pos, block_id) in chunk_blocks {
                blocks.push((local_pos.to_world(*chunk_pos), *block_id));
            }
        }

        blocks.sort_by_key(|(pos, _)| (pos.y, pos.z, pos.x));
        blocks
    }

    pub fn blocks_in_chunk(&self, chunk: ChunkPos) -> Vec<(BlockPos, u16)> {
        let Some(chunk_blocks) = self.chunks.get(&chunk) else {
            return Vec::new();
        };

        let mut blocks = Vec::new();
        for (local_pos, block_id) in chunk_blocks {
            blocks.push((local_pos.to_world(chunk), *block_id));
        }

        blocks.sort_by_key(|(pos, _)| (pos.y, pos.z, pos.x));
        blocks
    }

    pub fn unload_chunk(&mut self, chunk: ChunkPos) {
        self.chunks.remove(&chunk);
        self.block_data.remove(&chunk);
    }

    pub fn place_block(&mut self, block: BlockPos, block_id: u16) {
        if block_id == 0 {
            self.break_block(block);
            return;
        }

        let chunk = ChunkPos::from_block(block);
        let local = LocalBlockPos::from_block(block);
        self.chunks
            .entry(chunk)
            .or_default()
            .insert(local, block_id);

        if !is_fluid_block_id(block_id) {
            self.remove_block_data_entry(chunk, local);
        }
    }

    pub fn break_block(&mut self, block: BlockPos) -> bool {
        let chunk = ChunkPos::from_block(block);
        let local = LocalBlockPos::from_block(block);

        let mut removed = false;
        let mut remove_chunk = false;

        if let Some(chunk_blocks) = self.chunks.get_mut(&chunk) {
            removed = chunk_blocks.remove(&local).is_some();
            remove_chunk = chunk_blocks.is_empty();
        }

        self.remove_block_data_entry(chunk, local);

        if remove_chunk {
            self.chunks.remove(&chunk);
            self.block_data.remove(&chunk);
        }

        removed
    }

    pub fn block_id(&self, block: BlockPos) -> u16 {
        let chunk = ChunkPos::from_block(block);
        let local = LocalBlockPos::from_block(block);

        self.chunks
            .get(&chunk)
            .and_then(|chunk_blocks| chunk_blocks.get(&local))
            .copied()
            .unwrap_or(0)
    }

    pub fn block_data(&self, block: BlockPos) -> u8 {
        let chunk = ChunkPos::from_block(block);
        let local = LocalBlockPos::from_block(block);

        self.block_data
            .get(&chunk)
            .and_then(|chunk_data| chunk_data.get(&local))
            .copied()
            .unwrap_or(0)
    }

    pub fn set_block_data(&mut self, block: BlockPos, data: u8) {
        if self.block_id(block) == 0 {
            return;
        }

        let chunk = ChunkPos::from_block(block);
        let local = LocalBlockPos::from_block(block);

        if data == 0 {
            self.remove_block_data_entry(chunk, local);
            return;
        }

        self.block_data
            .entry(chunk)
            .or_default()
            .insert(local, data);
    }

    pub fn save_chunk(
        &self,
        world_root: impl AsRef<Path>,
        chunk: ChunkPos,
    ) -> Result<(), BlockWorldError> {
        let empty = ChunkBlocks::new();
        let empty_data = ChunkBlockData::new();
        let chunk_blocks = self.chunks.get(&chunk).unwrap_or(&empty);
        let chunk_data = self.block_data.get(&chunk).unwrap_or(&empty_data);
        let payload = serialize_chunk(chunk, chunk_blocks, chunk_data)?;
        save_chunk_payload(world_root, chunk.x, chunk.z, &payload)?;

        Ok(())
    }

    pub fn save_all_touched_chunks(
        &self,
        world_root: impl AsRef<Path>,
    ) -> Result<(), BlockWorldError> {
        let world_root = world_root.as_ref();
        for chunk in self.touched_chunks() {
            self.save_chunk(world_root, chunk)?;
        }

        Ok(())
    }

    pub fn load_chunk(
        &mut self,
        world_root: impl AsRef<Path>,
        chunk: ChunkPos,
    ) -> Result<(), BlockWorldError> {
        let payload = load_chunk_payload(world_root, chunk.x, chunk.z)?;
        match payload {
            None => {
                self.chunks.remove(&chunk);
                self.block_data.remove(&chunk);
                Ok(())
            }
            Some(bytes) => self.load_chunk_from_payload(chunk, &bytes),
        }
    }

    pub fn load_chunk_from_payload(
        &mut self,
        chunk: ChunkPos,
        payload: &[u8],
    ) -> Result<(), BlockWorldError> {
        let (chunk_blocks, chunk_data) = deserialize_chunk(chunk, payload)?;
        if chunk_blocks.is_empty() {
            self.chunks.remove(&chunk);
            self.block_data.remove(&chunk);
        } else {
            self.chunks.insert(chunk, chunk_blocks);
            if chunk_data.is_empty() {
                self.block_data.remove(&chunk);
            } else {
                self.block_data.insert(chunk, chunk_data);
            }
        }

        Ok(())
    }

    pub fn replace_chunk_blocks(&mut self, chunk: ChunkPos, blocks: Vec<(BlockPos, u16)>) {
        let mut chunk_blocks = ChunkBlocks::with_capacity(blocks.len());

        for (position, block_id) in blocks {
            if block_id == 0 || ChunkPos::from_block(position) != chunk {
                continue;
            }

            chunk_blocks.insert(LocalBlockPos::from_block(position), block_id);
        }

        if chunk_blocks.is_empty() {
            self.chunks.remove(&chunk);
            self.block_data.remove(&chunk);
        } else {
            self.chunks.insert(chunk, chunk_blocks);
            self.block_data.remove(&chunk);
        }
    }

    fn remove_block_data_entry(&mut self, chunk: ChunkPos, local: LocalBlockPos) {
        let mut remove_chunk_data = false;

        if let Some(chunk_data) = self.block_data.get_mut(&chunk) {
            chunk_data.remove(&local);
            remove_chunk_data = chunk_data.is_empty();
        }

        if remove_chunk_data {
            self.block_data.remove(&chunk);
        }
    }

    pub fn load_chunk_or_generate<F>(
        &mut self,
        world_root: impl AsRef<Path>,
        chunk: ChunkPos,
        mut generate: F,
    ) -> Result<ChunkLoadOutcome, BlockWorldError>
    where
        F: FnMut(ChunkPos) -> Vec<(BlockPos, u16)>,
    {
        let world_root = world_root.as_ref();
        let payload = load_chunk_payload(world_root, chunk.x, chunk.z)?;

        if let Some(bytes) = payload {
            self.load_chunk_from_payload(chunk, &bytes)?;
            return Ok(ChunkLoadOutcome::LoadedFromStorage);
        }

        self.unload_chunk(chunk);
        for (position, block_id) in generate(chunk) {
            if block_id == 0 {
                continue;
            }

            if ChunkPos::from_block(position) != chunk {
                continue;
            }

            self.place_block(position, block_id);
        }

        self.save_chunk(world_root, chunk)?;
        Ok(ChunkLoadOutcome::GeneratedFallback)
    }
}

fn serialize_chunk(
    chunk: ChunkPos,
    chunk_blocks: &ChunkBlocks,
    chunk_data: &ChunkBlockData,
) -> Result<Vec<u8>, BlockWorldError> {
    let mut root_compound = NbtCompound::new();
    root_compound.insert("ChunkX", NbtTag::Int(chunk.x));
    root_compound.insert("ChunkZ", NbtTag::Int(chunk.z));

    let mut blocks_list = NbtList::empty();
    let mut ordered: Vec<_> = chunk_blocks.iter().collect();
    ordered.sort_by_key(|(local, _)| (local.y, local.z, local.x));

    for (local, block_id) in ordered {
        let mut block_tag = NbtCompound::new();
        block_tag.insert(
            "X",
            NbtTag::Byte(i8::try_from(local.x).expect("local x should fit i8")),
        );
        block_tag.insert("Y", NbtTag::Int(local.y));
        block_tag.insert(
            "Z",
            NbtTag::Byte(i8::try_from(local.z).expect("local z should fit i8")),
        );
        block_tag.insert("Id", NbtTag::Int(i32::from(*block_id)));

        if let Some(data) = chunk_data.get(local)
            && *data != 0
        {
            block_tag.insert("Data", NbtTag::Int(i32::from(*data)));
        }

        blocks_list.push(NbtTag::Compound(block_tag))?;
    }

    root_compound.insert("Blocks", NbtTag::List(blocks_list));

    let root = NbtRoot::new("Chunk", root_compound);
    Ok(write_root_to_bytes(&root)?)
}

fn deserialize_chunk(
    expected_chunk: ChunkPos,
    payload: &[u8],
) -> Result<(ChunkBlocks, ChunkBlockData), BlockWorldError> {
    let root = read_root_from_bytes(payload)?;

    let chunk_x = read_int_field(&root.compound, "ChunkX")?;
    let chunk_z = read_int_field(&root.compound, "ChunkZ")?;
    let found_chunk = ChunkPos::new(chunk_x, chunk_z);
    if found_chunk != expected_chunk {
        return Err(BlockWorldError::ChunkMismatch {
            expected: expected_chunk,
            found: found_chunk,
        });
    }

    let blocks_tag = root
        .compound
        .get("Blocks")
        .ok_or(BlockWorldError::MissingField("Blocks"))?;

    let mut blocks = ChunkBlocks::new();
    let mut block_data = ChunkBlockData::new();
    let blocks_list = match blocks_tag {
        NbtTag::List(list) => list,
        _ => {
            return Err(BlockWorldError::InvalidFieldType {
                field: "Blocks",
                expected: TagType::List,
            });
        }
    };

    for entry in &blocks_list.elements {
        let block_compound = match entry {
            NbtTag::Compound(compound) => compound,
            _ => return Err(BlockWorldError::InvalidListEntryType),
        };

        let x = read_byte_field(block_compound, "X")?;
        let y = read_int_field(block_compound, "Y")?;
        let z = read_byte_field(block_compound, "Z")?;
        let block_id_i32 = read_int_field(block_compound, "Id")?;
        let block_id = u16::try_from(block_id_i32)
            .map_err(|_| BlockWorldError::InvalidBlockId(block_id_i32))?;

        if block_id == 0 {
            continue;
        }

        let local = LocalBlockPos::from_nbt(x, y, z)?;
        blocks.insert(local, block_id);

        let data_value = match block_compound.get("Data") {
            Some(NbtTag::Int(value)) => {
                u8::try_from(*value).map_err(|_| BlockWorldError::InvalidBlockId(*value))?
            }
            Some(_) => {
                return Err(BlockWorldError::InvalidFieldType {
                    field: "Data",
                    expected: TagType::Int,
                });
            }
            None => 0,
        };

        if data_value != 0 {
            block_data.insert(local, data_value);
        }
    }

    Ok((blocks, block_data))
}

pub fn decode_chunk_payload_to_blocks(
    expected_chunk: ChunkPos,
    payload: &[u8],
) -> Result<Vec<(BlockPos, u16)>, BlockWorldError> {
    let (chunk_blocks, _) = deserialize_chunk(expected_chunk, payload)?;
    let mut blocks = Vec::with_capacity(chunk_blocks.len());

    for (local, block_id) in chunk_blocks {
        blocks.push((local.to_world(expected_chunk), block_id));
    }

    Ok(blocks)
}

fn read_int_field(compound: &NbtCompound, field: &'static str) -> Result<i32, BlockWorldError> {
    match compound.get(field) {
        Some(NbtTag::Int(value)) => Ok(*value),
        Some(_) => Err(BlockWorldError::InvalidFieldType {
            field,
            expected: TagType::Int,
        }),
        None => Err(BlockWorldError::MissingField(field)),
    }
}

fn read_byte_field(compound: &NbtCompound, field: &'static str) -> Result<i8, BlockWorldError> {
    match compound.get(field) {
        Some(NbtTag::Byte(value)) => Ok(*value),
        Some(_) => Err(BlockWorldError::InvalidFieldType {
            field,
            expected: TagType::Byte,
        }),
        None => Err(BlockWorldError::MissingField(field)),
    }
}

fn is_fluid_block_id(block_id: u16) -> bool {
    matches!(block_id, 8 | 9 | 10 | 11)
}

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::save::nbt::{
    NbtCompound, NbtError, NbtRoot, NbtTag, TagType, read_root_from_bytes, write_root_to_bytes,
};
use crate::save::region::{RegionError, RegionFile};
use crate::world::WorldSnapshot;

const LEVEL_DAT_FILE: &str = "level.dat";

#[derive(Debug)]
pub enum WorldIoError {
    Io(io::Error),
    Nbt(NbtError),
    Region(RegionError),
    MissingField(&'static str),
    InvalidFieldType {
        field: &'static str,
        expected: TagType,
    },
    NegativeTickCount(i64),
    TickCountTooLarge(u64),
}

impl fmt::Display for WorldIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Nbt(error) => write!(f, "nbt error: {error}"),
            Self::Region(error) => write!(f, "region error: {error}"),
            Self::MissingField(field) => write!(f, "missing required field: {field}"),
            Self::InvalidFieldType { field, expected } => {
                write!(f, "invalid field type for {field}, expected {expected:?}")
            }
            Self::NegativeTickCount(tick_count) => {
                write!(f, "tick count cannot be negative: {tick_count}")
            }
            Self::TickCountTooLarge(tick_count) => {
                write!(f, "tick count exceeds i64::MAX: {tick_count}")
            }
        }
    }
}

impl std::error::Error for WorldIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Nbt(error) => Some(error),
            Self::Region(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for WorldIoError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<NbtError> for WorldIoError {
    fn from(error: NbtError) -> Self {
        Self::Nbt(error)
    }
}

impl From<RegionError> for WorldIoError {
    fn from(error: RegionError) -> Self {
        Self::Region(error)
    }
}

pub fn save_world_snapshot(
    world_root: impl AsRef<Path>,
    snapshot: &WorldSnapshot,
) -> Result<(), WorldIoError> {
    let world_root = world_root.as_ref();
    std::fs::create_dir_all(world_root)?;

    let root = snapshot_to_nbt(snapshot)?;
    let bytes = write_root_to_bytes(&root)?;
    std::fs::write(world_metadata_path(world_root), bytes)?;

    Ok(())
}

pub fn load_world_snapshot(world_root: impl AsRef<Path>) -> Result<WorldSnapshot, WorldIoError> {
    let world_root = world_root.as_ref();
    let bytes = std::fs::read(world_metadata_path(world_root))?;
    let root = read_root_from_bytes(&bytes)?;

    nbt_to_snapshot(&root)
}

pub fn save_chunk_payload(
    world_root: impl AsRef<Path>,
    chunk_x: i32,
    chunk_z: i32,
    payload: &[u8],
) -> Result<(), WorldIoError> {
    let world_root = world_root.as_ref();
    std::fs::create_dir_all(world_root)?;

    let (region_path, local_x, local_z) = region_path_for_chunk(world_root, chunk_x, chunk_z);
    let mut region = RegionFile::open(region_path)?;
    region.write_chunk(local_x, local_z, payload)?;

    Ok(())
}

pub fn load_chunk_payload(
    world_root: impl AsRef<Path>,
    chunk_x: i32,
    chunk_z: i32,
) -> Result<Option<Vec<u8>>, WorldIoError> {
    let world_root = world_root.as_ref();
    let (region_path, local_x, local_z) = region_path_for_chunk(world_root, chunk_x, chunk_z);
    if !region_path.exists() {
        return Ok(None);
    }

    let mut region = RegionFile::open(region_path)?;
    Ok(region.read_chunk(local_x, local_z)?)
}

fn world_metadata_path(world_root: &Path) -> PathBuf {
    world_root.join(LEVEL_DAT_FILE)
}

fn region_path_for_chunk(world_root: &Path, chunk_x: i32, chunk_z: i32) -> (PathBuf, i32, i32) {
    let region_x = chunk_x.div_euclid(32);
    let region_z = chunk_z.div_euclid(32);
    let local_x = chunk_x.rem_euclid(32);
    let local_z = chunk_z.rem_euclid(32);

    let path = world_root
        .join("region")
        .join(format!("r.{region_x}.{region_z}.mcr"));
    (path, local_x, local_z)
}

fn snapshot_to_nbt(snapshot: &WorldSnapshot) -> Result<NbtRoot, WorldIoError> {
    let tick_count = i64::try_from(snapshot.tick_count)
        .map_err(|_| WorldIoError::TickCountTooLarge(snapshot.tick_count))?;

    let mut data = NbtCompound::new();
    data.insert("LevelName", NbtTag::String(snapshot.name.clone()));
    data.insert("RandomSeed", NbtTag::Long(snapshot.seed));
    data.insert("TickCount", NbtTag::Long(tick_count));

    Ok(NbtRoot::new("Data", data))
}

fn nbt_to_snapshot(root: &NbtRoot) -> Result<WorldSnapshot, WorldIoError> {
    let name = match root.compound.get("LevelName") {
        Some(NbtTag::String(value)) => value.clone(),
        Some(_) => {
            return Err(WorldIoError::InvalidFieldType {
                field: "LevelName",
                expected: TagType::String,
            });
        }
        None => return Err(WorldIoError::MissingField("LevelName")),
    };

    let seed = match root.compound.get("RandomSeed") {
        Some(NbtTag::Long(value)) => *value,
        Some(_) => {
            return Err(WorldIoError::InvalidFieldType {
                field: "RandomSeed",
                expected: TagType::Long,
            });
        }
        None => return Err(WorldIoError::MissingField("RandomSeed")),
    };

    let tick_count = match root.compound.get("TickCount") {
        Some(NbtTag::Long(value)) => *value,
        Some(_) => {
            return Err(WorldIoError::InvalidFieldType {
                field: "TickCount",
                expected: TagType::Long,
            });
        }
        None => return Err(WorldIoError::MissingField("TickCount")),
    };

    if tick_count < 0 {
        return Err(WorldIoError::NegativeTickCount(tick_count));
    }

    Ok(WorldSnapshot {
        name,
        seed,
        tick_count: u64::try_from(tick_count).expect("negative value already rejected"),
    })
}

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SECTOR_BYTES: usize = 4096;
pub const CHUNK_GRID_SIZE: usize = 32;
pub const HEADER_SECTORS: usize = 2;
pub const HEADER_BYTES: usize = SECTOR_BYTES * HEADER_SECTORS;
const CHUNK_HEADER_BYTES: usize = 8;
const MAX_SECTORS_PER_CHUNK: usize = 255;

#[derive(Debug)]
pub enum RegionError {
    Io(io::Error),
    OutOfBounds {
        x: i32,
        z: i32,
    },
    ChunkTooLarge {
        payload_bytes: usize,
    },
    InvalidOffset(u32),
    InvalidLength {
        x: i32,
        z: i32,
        length: u32,
        sector_count: u32,
    },
    UnsupportedCompressionFlag {
        x: i32,
        z: i32,
    },
}

impl fmt::Display for RegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::OutOfBounds { x, z } => write!(f, "chunk coordinates out of bounds: ({x}, {z})"),
            Self::ChunkTooLarge { payload_bytes } => {
                write!(
                    f,
                    "chunk payload too large for region format: {payload_bytes} bytes"
                )
            }
            Self::InvalidOffset(offset) => write!(f, "invalid region offset entry: {offset:#010x}"),
            Self::InvalidLength {
                x,
                z,
                length,
                sector_count,
            } => write!(
                f,
                "invalid chunk length for ({x}, {z}): {length} bytes for {sector_count} sectors"
            ),
            Self::UnsupportedCompressionFlag { x, z } => write!(
                f,
                "chunk ({x}, {z}) uses unsupported compressed payload flag"
            ),
        }
    }
}

impl std::error::Error for RegionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for RegionError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct RegionFile {
    path: PathBuf,
    file: File,
    offsets: [u32; CHUNK_GRID_SIZE * CHUNK_GRID_SIZE],
    timestamps: [u32; CHUNK_GRID_SIZE * CHUNK_GRID_SIZE],
    sector_free: Vec<bool>,
}

impl RegionFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RegionError> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        let mut file_len =
            usize::try_from(file.metadata()?.len()).map_err(|_| RegionError::ChunkTooLarge {
                payload_bytes: usize::MAX,
            })?;

        if file_len < HEADER_BYTES {
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&vec![0_u8; HEADER_BYTES])?;
            file_len = HEADER_BYTES;
        }

        if file_len % SECTOR_BYTES != 0 {
            let padding = SECTOR_BYTES - (file_len % SECTOR_BYTES);
            file.seek(SeekFrom::End(0))?;
            file.write_all(&vec![0_u8; padding])?;
            file_len += padding;
        }

        file.seek(SeekFrom::Start(0))?;

        let mut offsets = [0_u32; CHUNK_GRID_SIZE * CHUNK_GRID_SIZE];
        for offset in &mut offsets {
            *offset = read_u32_be(&mut file)?;
        }

        let mut timestamps = [0_u32; CHUNK_GRID_SIZE * CHUNK_GRID_SIZE];
        for timestamp in &mut timestamps {
            *timestamp = read_u32_be(&mut file)?;
        }

        let sector_count = file_len / SECTOR_BYTES;
        let mut sector_free = vec![true; sector_count.max(HEADER_SECTORS)];
        sector_free[0] = false;
        sector_free[1] = false;

        for offset in offsets {
            if offset == 0 {
                continue;
            }

            let sector_number =
                usize::try_from(offset >> 8).map_err(|_| RegionError::InvalidOffset(offset))?;
            let sectors_used =
                usize::try_from(offset & 0xFF).map_err(|_| RegionError::InvalidOffset(offset))?;

            if sector_number < HEADER_SECTORS || sectors_used == 0 {
                return Err(RegionError::InvalidOffset(offset));
            }

            let end = sector_number.saturating_add(sectors_used);
            if end > sector_free.len() {
                return Err(RegionError::InvalidOffset(offset));
            }

            for slot in &mut sector_free[sector_number..end] {
                *slot = false;
            }
        }

        Ok(Self {
            path,
            file,
            offsets,
            timestamps,
            sector_free,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn has_chunk(&self, x: i32, z: i32) -> Result<bool, RegionError> {
        let index = Self::chunk_index(x, z)?;
        Ok(self.offsets[index] != 0)
    }

    pub fn read_chunk(&mut self, x: i32, z: i32) -> Result<Option<Vec<u8>>, RegionError> {
        let index = Self::chunk_index(x, z)?;
        let offset = self.offsets[index];
        if offset == 0 {
            return Ok(None);
        }

        let sector_number = offset >> 8;
        let sector_count = offset & 0xFF;
        if sector_number == 0 || sector_count == 0 {
            return Err(RegionError::InvalidOffset(offset));
        }

        let chunk_start = u64::from(sector_number)
            .checked_mul(u64::try_from(SECTOR_BYTES).expect("sector bytes should fit u64"))
            .ok_or(RegionError::InvalidOffset(offset))?;
        self.file.seek(SeekFrom::Start(chunk_start))?;

        let mut length = read_u32_be(&mut self.file)?;
        let compressed = (length & 0x8000_0000) != 0;
        if compressed {
            return Err(RegionError::UnsupportedCompressionFlag { x, z });
        }

        length &= 0x7FFF_FFFF;
        let _decompressed_length = read_u32_be(&mut self.file)?;

        let max_payload = sector_count
            .checked_mul(u32::try_from(SECTOR_BYTES).expect("sector bytes should fit u32"))
            .and_then(|value| value.checked_sub(u32::try_from(CHUNK_HEADER_BYTES).unwrap_or(8)))
            .ok_or(RegionError::InvalidOffset(offset))?;

        if length > max_payload {
            return Err(RegionError::InvalidLength {
                x,
                z,
                length,
                sector_count,
            });
        }

        let payload_len = usize::try_from(length).map_err(|_| RegionError::InvalidLength {
            x,
            z,
            length,
            sector_count,
        })?;
        let mut payload = vec![0_u8; payload_len];
        self.file.read_exact(&mut payload)?;

        Ok(Some(payload))
    }

    pub fn write_chunk(&mut self, x: i32, z: i32, payload: &[u8]) -> Result<(), RegionError> {
        let index = Self::chunk_index(x, z)?;

        let payload_with_header =
            payload
                .len()
                .checked_add(CHUNK_HEADER_BYTES)
                .ok_or(RegionError::ChunkTooLarge {
                    payload_bytes: payload.len(),
                })?;

        let sectors_needed = payload_with_header.div_ceil(SECTOR_BYTES);
        if sectors_needed == 0 || sectors_needed > MAX_SECTORS_PER_CHUNK {
            return Err(RegionError::ChunkTooLarge {
                payload_bytes: payload.len(),
            });
        }

        let current_offset = self.offsets[index];
        let current_sector = usize::try_from(current_offset >> 8)
            .map_err(|_| RegionError::InvalidOffset(current_offset))?;
        let current_count = usize::try_from(current_offset & 0xFF)
            .map_err(|_| RegionError::InvalidOffset(current_offset))?;

        let target_sector = if current_offset != 0 && current_count == sectors_needed {
            current_sector
        } else {
            if current_offset != 0 {
                self.free_sectors(current_sector, current_count)?;
            }

            if let Some(sector) = self.find_free_run(sectors_needed) {
                self.mark_allocated(sector, sectors_needed)?;
                sector
            } else {
                self.expand_for(sectors_needed)?
            }
        };

        self.write_chunk_at(target_sector, sectors_needed, payload)?;

        let sector_u32 = u32::try_from(target_sector).map_err(|_| RegionError::ChunkTooLarge {
            payload_bytes: payload.len(),
        })?;
        let sectors_u32 =
            u32::try_from(sectors_needed).map_err(|_| RegionError::ChunkTooLarge {
                payload_bytes: payload.len(),
            })?;

        self.offsets[index] = (sector_u32 << 8) | sectors_u32;
        self.timestamps[index] = current_unix_timestamp();
        self.write_header_entry(index)?;

        Ok(())
    }

    fn write_chunk_at(
        &mut self,
        sector: usize,
        sectors_used: usize,
        payload: &[u8],
    ) -> Result<(), RegionError> {
        let chunk_start = u64::try_from(sector)
            .expect("sector index should fit u64")
            .checked_mul(u64::try_from(SECTOR_BYTES).expect("sector bytes should fit u64"))
            .ok_or(RegionError::ChunkTooLarge {
                payload_bytes: payload.len(),
            })?;
        self.file.seek(SeekFrom::Start(chunk_start))?;

        let payload_len_u32 =
            u32::try_from(payload.len()).map_err(|_| RegionError::ChunkTooLarge {
                payload_bytes: payload.len(),
            })?;
        write_u32_be(&mut self.file, payload_len_u32)?;
        write_u32_be(&mut self.file, payload_len_u32)?;
        self.file.write_all(payload)?;

        let allocated_bytes =
            sectors_used
                .checked_mul(SECTOR_BYTES)
                .ok_or(RegionError::ChunkTooLarge {
                    payload_bytes: payload.len(),
                })?;
        let written =
            CHUNK_HEADER_BYTES
                .checked_add(payload.len())
                .ok_or(RegionError::ChunkTooLarge {
                    payload_bytes: payload.len(),
                })?;

        if allocated_bytes > written {
            self.file
                .write_all(&vec![0_u8; allocated_bytes - written])?;
        }

        Ok(())
    }

    fn write_header_entry(&mut self, index: usize) -> Result<(), RegionError> {
        let offset_pos = u64::try_from(index)
            .expect("index should fit u64")
            .checked_mul(4)
            .expect("offset position multiplication should not overflow");
        self.file.seek(SeekFrom::Start(offset_pos))?;
        write_u32_be(&mut self.file, self.offsets[index])?;

        let timestamp_pos = u64::try_from(SECTOR_BYTES)
            .expect("sector bytes should fit u64")
            .checked_add(offset_pos)
            .expect("timestamp position should not overflow");
        self.file.seek(SeekFrom::Start(timestamp_pos))?;
        write_u32_be(&mut self.file, self.timestamps[index])?;

        self.file.flush()?;
        Ok(())
    }

    fn find_free_run(&self, sectors_needed: usize) -> Option<usize> {
        let mut run_start = 0_usize;
        let mut run_length = 0_usize;

        for (index, is_free) in self.sector_free.iter().copied().enumerate() {
            if is_free {
                if run_length == 0 {
                    run_start = index;
                }

                run_length += 1;
                if run_length >= sectors_needed {
                    return Some(run_start);
                }
            } else {
                run_length = 0;
            }
        }

        None
    }

    fn mark_allocated(&mut self, start: usize, count: usize) -> Result<(), RegionError> {
        let end = start
            .checked_add(count)
            .ok_or(RegionError::ChunkTooLarge { payload_bytes: 0 })?;
        if end > self.sector_free.len() {
            return Err(RegionError::ChunkTooLarge { payload_bytes: 0 });
        }

        for slot in &mut self.sector_free[start..end] {
            *slot = false;
        }

        Ok(())
    }

    fn free_sectors(&mut self, start: usize, count: usize) -> Result<(), RegionError> {
        if count == 0 {
            return Ok(());
        }

        let end = start
            .checked_add(count)
            .ok_or(RegionError::ChunkTooLarge { payload_bytes: 0 })?;
        if end > self.sector_free.len() {
            return Err(RegionError::ChunkTooLarge { payload_bytes: 0 });
        }

        for slot in &mut self.sector_free[start..end] {
            *slot = true;
        }

        Ok(())
    }

    fn expand_for(&mut self, sectors_needed: usize) -> Result<usize, RegionError> {
        let new_start = self.sector_free.len();
        self.file.seek(SeekFrom::End(0))?;
        self.file
            .write_all(&vec![0_u8; sectors_needed * SECTOR_BYTES])?;
        self.sector_free
            .extend(std::iter::repeat_n(false, sectors_needed));

        Ok(new_start)
    }

    fn chunk_index(x: i32, z: i32) -> Result<usize, RegionError> {
        if !(0..CHUNK_GRID_SIZE as i32).contains(&x) || !(0..CHUNK_GRID_SIZE as i32).contains(&z) {
            return Err(RegionError::OutOfBounds { x, z });
        }

        let x = usize::try_from(x).expect("bounds checked");
        let z = usize::try_from(z).expect("bounds checked");

        Ok(x + z * CHUNK_GRID_SIZE)
    }
}

fn read_u32_be<R: Read>(reader: &mut R) -> Result<u32, RegionError> {
    let mut buffer = [0_u8; 4];
    reader.read_exact(&mut buffer)?;
    Ok(u32::from_be_bytes(buffer))
}

fn write_u32_be<W: Write>(writer: &mut W, value: u32) -> Result<(), RegionError> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn current_unix_timestamp() -> u32 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    u32::try_from(seconds).unwrap_or(u32::MAX)
}

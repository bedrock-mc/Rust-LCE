use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use crate::save::world_io::load_chunk_payload;
use crate::world::blocks::decode_chunk_payload_to_blocks;
use crate::world::worldgen::RandomLevelSource;
use crate::world::{BlockPos, ChunkPos};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkDataSource {
    Storage,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedChunk {
    pub chunk: ChunkPos,
    pub blocks: Vec<(BlockPos, u16)>,
    pub source: ChunkDataSource,
}

#[derive(Debug)]
enum WorldWorkerCommand {
    RequestChunk { chunk: ChunkPos },
    CancelChunk { chunk: ChunkPos },
    Shutdown,
}

#[derive(Debug)]
pub struct WorldWorker {
    request_tx: Sender<WorldWorkerCommand>,
    result_rx: Receiver<GeneratedChunk>,
    pending_chunks: HashSet<ChunkPos>,
    ready_chunks: VecDeque<GeneratedChunk>,
    thread_handle: Option<JoinHandle<()>>,
}

impl WorldWorker {
    pub fn spawn(world_seed: i64) -> Self {
        Self::spawn_inner(world_seed, None)
    }

    pub fn spawn_with_save_root(world_seed: i64, save_root: PathBuf) -> Self {
        Self::spawn_inner(world_seed, Some(save_root))
    }

    fn spawn_inner(world_seed: i64, save_root: Option<PathBuf>) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<WorldWorkerCommand>();
        let (result_tx, result_rx) = mpsc::channel::<GeneratedChunk>();

        let thread_handle = thread::Builder::new()
            .name("lce_world_worker".to_string())
            .spawn(move || run_worker_loop(world_seed, save_root, request_rx, result_tx))
            .expect("world worker thread should start");

        Self {
            request_tx,
            result_rx,
            pending_chunks: HashSet::new(),
            ready_chunks: VecDeque::new(),
            thread_handle: Some(thread_handle),
        }
    }

    pub fn request_chunk_generation(&mut self, chunk: ChunkPos) -> bool {
        if !self.pending_chunks.insert(chunk) {
            return false;
        }

        if self
            .request_tx
            .send(WorldWorkerCommand::RequestChunk { chunk })
            .is_err()
        {
            self.pending_chunks.remove(&chunk);
            return false;
        }

        true
    }

    pub fn cancel_chunk_interest(&mut self, chunk: ChunkPos) {
        if self.pending_chunks.remove(&chunk) {
            self.ready_chunks.retain(|entry| entry.chunk != chunk);
            let _ = self
                .request_tx
                .send(WorldWorkerCommand::CancelChunk { chunk });
        }
    }

    pub fn is_chunk_pending(&self, chunk: ChunkPos) -> bool {
        self.pending_chunks.contains(&chunk)
    }

    pub fn pending_count(&self) -> usize {
        self.pending_chunks.len()
    }

    pub fn poll_generated_chunks(&mut self) -> Vec<GeneratedChunk> {
        self.poll_generated_chunks_with_limit(usize::MAX)
    }

    pub fn poll_generated_chunks_with_limit(&mut self, max_chunks: usize) -> Vec<GeneratedChunk> {
        if max_chunks == 0 {
            return Vec::new();
        }

        loop {
            match self.result_rx.try_recv() {
                Ok(result) => {
                    if self.pending_chunks.contains(&result.chunk) {
                        self.ready_chunks.push_back(result);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        let to_take = max_chunks.min(self.ready_chunks.len());
        let mut ready = Vec::with_capacity(to_take);
        for _ in 0..to_take {
            if let Some(chunk) = self.ready_chunks.pop_front() {
                self.pending_chunks.remove(&chunk.chunk);
                ready.push(chunk);
            }
        }

        ready
    }
}

impl Drop for WorldWorker {
    fn drop(&mut self) {
        let _ = self.request_tx.send(WorldWorkerCommand::Shutdown);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_worker_loop(
    world_seed: i64,
    save_root: Option<PathBuf>,
    request_rx: Receiver<WorldWorkerCommand>,
    result_tx: Sender<GeneratedChunk>,
) {
    let generator = RandomLevelSource::new(world_seed);
    let mut requested_chunks = HashSet::new();
    let mut request_queue = VecDeque::new();

    loop {
        if requested_chunks.is_empty() {
            match request_rx.recv() {
                Ok(command) => {
                    if apply_worker_command(command, &mut requested_chunks, &mut request_queue) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        loop {
            match request_rx.try_recv() {
                Ok(command) => {
                    if apply_worker_command(command, &mut requested_chunks, &mut request_queue) {
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        let Some(chunk) = next_requested_chunk(&mut requested_chunks, &mut request_queue) else {
            continue;
        };

        let generated = load_or_generate_chunk(&generator, save_root.as_deref(), chunk);
        if result_tx.send(generated).is_err() {
            break;
        }
    }
}

fn load_or_generate_chunk(
    generator: &RandomLevelSource,
    save_root: Option<&Path>,
    chunk: ChunkPos,
) -> GeneratedChunk {
    if let Some(save_root) = save_root
        && let Ok(Some(payload)) = load_chunk_payload(save_root, chunk.x, chunk.z)
        && let Ok(blocks) = decode_chunk_payload_to_blocks(chunk, &payload)
    {
        return GeneratedChunk {
            chunk,
            blocks,
            source: ChunkDataSource::Storage,
        };
    }

    let generated = generator.generate_chunk(chunk);
    GeneratedChunk {
        chunk,
        blocks: generated.blocks,
        source: ChunkDataSource::Generated,
    }
}

fn apply_worker_command(
    command: WorldWorkerCommand,
    requested_chunks: &mut HashSet<ChunkPos>,
    request_queue: &mut VecDeque<ChunkPos>,
) -> bool {
    match command {
        WorldWorkerCommand::RequestChunk { chunk } => {
            if requested_chunks.insert(chunk) {
                request_queue.push_back(chunk);
            }
            false
        }
        WorldWorkerCommand::CancelChunk { chunk } => {
            requested_chunks.remove(&chunk);
            false
        }
        WorldWorkerCommand::Shutdown => true,
    }
}

fn next_requested_chunk(
    requested_chunks: &mut HashSet<ChunkPos>,
    request_queue: &mut VecDeque<ChunkPos>,
) -> Option<ChunkPos> {
    while let Some(chunk) = request_queue.pop_front() {
        if requested_chunks.remove(&chunk) {
            return Some(chunk);
        }
    }

    None
}

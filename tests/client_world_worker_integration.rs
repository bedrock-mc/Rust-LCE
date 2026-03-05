use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use lce_rust::client::world_worker::{ChunkDataSource, GeneratedChunk, WorldWorker};
use lce_rust::world::{BlockPos, BlockWorld, ChunkPos};

#[test]
fn requests_are_deduplicated_until_result_is_drained() {
    let mut worker = WorldWorker::spawn(12_345);
    let chunk = ChunkPos::new(2, -1);

    assert!(worker.request_chunk_generation(chunk));
    assert!(!worker.request_chunk_generation(chunk));
    assert_eq!(worker.pending_count(), 1);

    let generated = wait_for_generated_chunks(&mut worker, 1, Duration::from_secs(3));
    assert_eq!(generated[0].chunk, chunk);
    assert!(!generated[0].blocks.is_empty());

    assert!(worker.request_chunk_generation(chunk));
}

#[test]
fn cancelled_chunk_interest_drops_stale_worker_results() {
    let mut worker = WorldWorker::spawn(90_001);
    let chunk = ChunkPos::new(-3, 4);

    assert!(worker.request_chunk_generation(chunk));
    worker.cancel_chunk_interest(chunk);

    let generated = wait_for_generated_chunks(&mut worker, 1, Duration::from_millis(400));
    assert!(generated.is_empty());
}

#[test]
fn worker_can_generate_multiple_chunks_in_background() {
    let mut worker = WorldWorker::spawn(-777);
    let chunk_a = ChunkPos::new(0, 0);
    let chunk_b = ChunkPos::new(5, -2);

    assert!(worker.request_chunk_generation(chunk_a));
    assert!(worker.request_chunk_generation(chunk_b));

    let generated = wait_for_generated_chunks(&mut worker, 2, Duration::from_secs(3));
    assert_eq!(generated.len(), 2);
    assert!(generated.iter().any(|entry| entry.chunk == chunk_a));
    assert!(generated.iter().any(|entry| entry.chunk == chunk_b));
}

#[test]
fn pending_state_updates_as_chunks_are_requested_and_cancelled() {
    let mut worker = WorldWorker::spawn(333);
    let chunk = ChunkPos::new(-7, 9);

    assert!(!worker.is_chunk_pending(chunk));
    assert!(worker.request_chunk_generation(chunk));
    assert!(worker.is_chunk_pending(chunk));

    worker.cancel_chunk_interest(chunk);
    assert!(!worker.is_chunk_pending(chunk));
}

#[test]
fn poll_limit_caps_generated_chunks_per_call() {
    let mut worker = WorldWorker::spawn(42);
    let chunk_a = ChunkPos::new(0, 0);
    let chunk_b = ChunkPos::new(1, 0);
    let chunk_c = ChunkPos::new(2, 0);

    assert!(worker.request_chunk_generation(chunk_a));
    assert!(worker.request_chunk_generation(chunk_b));
    assert!(worker.request_chunk_generation(chunk_c));

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut collected = Vec::new();

    while Instant::now() < deadline && collected.len() < 3 {
        let batch = worker.poll_generated_chunks_with_limit(1);
        assert!(batch.len() <= 1);

        if let Some(chunk) = batch.into_iter().next() {
            collected.push(chunk.chunk);
        }

        if collected.len() < 3 {
            thread::sleep(Duration::from_millis(10));
        }
    }

    assert_eq!(collected.len(), 3);
    assert!(collected.contains(&chunk_a));
    assert!(collected.contains(&chunk_b));
    assert!(collected.contains(&chunk_c));
}

#[test]
fn limited_poll_keeps_unconsumed_results_marked_pending() {
    let mut worker = WorldWorker::spawn(73);
    let chunk_a = ChunkPos::new(11, 0);
    let chunk_b = ChunkPos::new(12, 0);

    assert!(worker.request_chunk_generation(chunk_a));
    assert!(worker.request_chunk_generation(chunk_b));

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        let first = worker.poll_generated_chunks_with_limit(1);
        if first.len() == 1 {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(worker.pending_count(), 1);

    let rest = wait_for_generated_chunks(&mut worker, 1, Duration::from_secs(2));
    assert_eq!(rest.len(), 1);
    assert_eq!(worker.pending_count(), 0);
}

#[test]
fn worker_prefers_storage_payload_when_save_root_is_configured() {
    let root = unique_temp_world_root("worker_storage_pref");
    fs::create_dir_all(&root).expect("temp world root should be created");

    let chunk = ChunkPos::new(0, 0);
    let marker = BlockPos::new(1, 64, 1);

    let mut world = BlockWorld::new();
    world.place_block(marker, 5);
    world
        .save_chunk(&root, chunk)
        .expect("chunk should be persisted for storage-backed worker read");

    let mut worker = WorldWorker::spawn_with_save_root(9001, root.clone());
    assert!(worker.request_chunk_generation(chunk));

    let generated = wait_for_generated_chunks(&mut worker, 1, Duration::from_secs(3));
    assert_eq!(generated.len(), 1);
    assert_eq!(generated[0].source, ChunkDataSource::Storage);
    assert!(
        generated[0]
            .blocks
            .iter()
            .any(|(position, block_id)| *position == marker && *block_id == 5)
    );

    let _ = fs::remove_dir_all(&root);
}

fn unique_temp_world_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("lce_rust_{label}_{nanos}"))
}

fn wait_for_generated_chunks(
    worker: &mut WorldWorker,
    expected_count: usize,
    timeout: Duration,
) -> Vec<GeneratedChunk> {
    let deadline = Instant::now() + timeout;
    let mut generated = Vec::new();

    while Instant::now() < deadline {
        generated.extend(worker.poll_generated_chunks());
        if generated.len() >= expected_count {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }

    generated
}

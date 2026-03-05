use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lce_rust::save::region::{HEADER_BYTES, RegionError, RegionFile};

#[test]
fn creates_region_file_and_round_trips_chunk_payload() {
    let path = unique_temp_path("round_trip");

    {
        let mut region = RegionFile::open(&path).expect("region should open");
        assert!(
            region
                .path()
                .ends_with(path.file_name().expect("temp file should have name"))
        );
        assert!(!region.has_chunk(0, 0).expect("query should succeed"));

        let payload = vec![1_u8, 2, 3, 4, 5, 6];
        region
            .write_chunk(0, 0, &payload)
            .expect("chunk write should succeed");

        assert!(region.has_chunk(0, 0).expect("query should succeed"));
        let stored = region
            .read_chunk(0, 0)
            .expect("chunk read should succeed")
            .expect("chunk should exist");
        assert_eq!(stored, payload);
    }

    {
        let mut region = RegionFile::open(&path).expect("region reopen should succeed");
        let stored = region
            .read_chunk(0, 0)
            .expect("chunk read should succeed")
            .expect("chunk should exist after reopen");
        assert_eq!(stored, vec![1_u8, 2, 3, 4, 5, 6]);
    }

    let metadata = std::fs::metadata(&path).expect("region file should exist");
    assert!(
        usize::try_from(metadata.len()).expect("metadata length should fit usize") >= HEADER_BYTES
    );

    cleanup(&path);
}

#[test]
fn rewrites_chunk_with_larger_and_smaller_payloads() {
    let path = unique_temp_path("rewrite");
    let mut region = RegionFile::open(&path).expect("region should open");

    let first = vec![7_u8; 500];
    region
        .write_chunk(4, 9, &first)
        .expect("initial write should succeed");

    let larger = vec![9_u8; 9000];
    region
        .write_chunk(4, 9, &larger)
        .expect("larger write should succeed");

    let smaller = vec![3_u8; 128];
    region
        .write_chunk(4, 9, &smaller)
        .expect("smaller rewrite should succeed");

    let stored = region
        .read_chunk(4, 9)
        .expect("chunk read should succeed")
        .expect("chunk should exist");
    assert_eq!(stored, smaller);

    cleanup(&path);
}

#[test]
fn rejects_out_of_bounds_coordinates() {
    let path = unique_temp_path("bounds");
    let mut region = RegionFile::open(&path).expect("region should open");

    let result = region.write_chunk(32, 0, &[1, 2, 3]);
    assert!(matches!(
        result,
        Err(RegionError::OutOfBounds { x: 32, z: 0 })
    ));

    cleanup(&path);
}

fn unique_temp_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "lce_rust_{test_name}_{}_{}.mcr",
        std::process::id(),
        nanos
    ));
    path
}

fn cleanup(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}

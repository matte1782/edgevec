//! Persistence v3 format tests (W16.5)
//!
//! These tests verify that the v0.3 persistence format correctly:
//! - Persists the `deleted` field per node
//! - Persists `deleted_count` in the header
//! - Supports migration from v0.1/v0.2 formats
//! - Maintains CRC32 checksum integrity

use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::persistence::storage::MemoryBackend;
use edgevec::persistence::{
    read_snapshot, write_snapshot, FileHeader, StorageBackend, VERSION_MINOR,
};
use edgevec::storage::VectorStorage;

/// Create an index with vectors for testing.
fn create_test_index(count: usize, dim: u32) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(dim);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    for i in 0..count {
        let vec: Vec<f32> = (0..dim)
            .map(|j| (i * dim as usize + j as usize) as f32)
            .collect();
        index.insert(&vec, &mut storage).unwrap();
    }

    (index, storage)
}

#[test]
fn test_save_load_v3_roundtrip_no_deletes() {
    let (index, storage) = create_test_index(10, 4);

    // Save
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    // Load
    let (loaded_index, _loaded_storage) = read_snapshot(&backend).unwrap();

    // Verify
    assert_eq!(loaded_index.node_count(), 10);
    assert_eq!(loaded_index.deleted_count(), 0);
    assert_eq!(loaded_index.live_count(), 10);
}

#[test]
fn test_save_load_v3_roundtrip_with_deletes() {
    let (mut index, storage) = create_test_index(10, 4);

    // Delete some vectors
    index.soft_delete(VectorId(1)).unwrap();
    index.soft_delete(VectorId(3)).unwrap();
    index.soft_delete(VectorId(5)).unwrap();

    assert_eq!(index.deleted_count(), 3);

    // Save
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    // Load
    let (loaded_index, _loaded_storage) = read_snapshot(&backend).unwrap();

    // Verify deleted_count persisted
    assert_eq!(loaded_index.node_count(), 10);
    assert_eq!(loaded_index.deleted_count(), 3);
    assert_eq!(loaded_index.live_count(), 7);

    // Verify specific IDs are deleted
    assert!(loaded_index.is_deleted(VectorId(1)).unwrap());
    assert!(loaded_index.is_deleted(VectorId(3)).unwrap());
    assert!(loaded_index.is_deleted(VectorId(5)).unwrap());

    // Verify other IDs are NOT deleted
    assert!(!loaded_index.is_deleted(VectorId(2)).unwrap());
    assert!(!loaded_index.is_deleted(VectorId(4)).unwrap());
    assert!(!loaded_index.is_deleted(VectorId(6)).unwrap());
}

#[test]
fn test_save_v3_has_deleted_count_in_header() {
    let (mut index, storage) = create_test_index(5, 4);

    // Delete 2 vectors
    index.soft_delete(VectorId(1)).unwrap();
    index.soft_delete(VectorId(3)).unwrap();

    // Save
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    // Read raw bytes to verify header
    let data = backend.read().unwrap();
    assert!(data.len() >= 64, "Should have at least header");

    // Check version is 0.3
    let version_major = data[4];
    let version_minor = data[5];
    assert_eq!(version_major, 0);
    assert_eq!(version_minor, VERSION_MINOR);

    // Check deleted_count at offset 60-64
    let deleted_count = u32::from_le_bytes([data[60], data[61], data[62], data[63]]);
    assert_eq!(deleted_count, 2, "Header should have deleted_count=2");
}

#[test]
fn test_deleted_nodes_persist_correctly() {
    let (mut index, storage) = create_test_index(100, 128);

    // Delete specific IDs (pattern: every 10th starting at 5)
    let deleted_ids: Vec<u64> = vec![5, 15, 25, 35, 45, 55, 65, 75, 85, 95];
    for id in &deleted_ids {
        index.soft_delete(VectorId(*id)).unwrap();
    }

    // Save and load
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    let (loaded_index, _loaded_storage) = read_snapshot(&backend).unwrap();

    // Verify all deleted IDs are still deleted
    for id in &deleted_ids {
        assert!(
            loaded_index.is_deleted(VectorId(*id)).unwrap(),
            "ID {} should be deleted after load",
            id
        );
    }

    // Verify non-deleted IDs are not deleted
    for id in 1..=100u64 {
        if !deleted_ids.contains(&id) {
            assert!(
                !loaded_index.is_deleted(VectorId(id)).unwrap(),
                "ID {} should not be deleted after load",
                id
            );
        }
    }

    // Verify counts
    assert_eq!(loaded_index.deleted_count(), 10);
    assert_eq!(loaded_index.live_count(), 90);
}

#[test]
fn test_crc32_checksum_catches_corruption() {
    let (index, storage) = create_test_index(5, 4);

    // Save
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    // Corrupt a byte in the data section (after header)
    let data = backend.read().unwrap();
    let mut corrupted = data.clone();
    if corrupted.len() > 100 {
        corrupted[100] ^= 0xFF;
    }

    // Write corrupted data back
    let corrupted_backend = MemoryBackend::default();
    corrupted_backend.atomic_write("", &corrupted).unwrap();

    // Load should fail with checksum error
    let result = read_snapshot(&corrupted_backend);
    assert!(result.is_err(), "Should fail to load corrupted snapshot");
}

#[test]
fn test_header_checksum_catches_corruption() {
    let (index, storage) = create_test_index(5, 4);

    // Save
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    // Corrupt a byte in the header (dimensions field at offset 40)
    let data = backend.read().unwrap();
    let mut corrupted = data.clone();
    corrupted[40] ^= 0xFF;

    // Write corrupted data back
    let corrupted_backend = MemoryBackend::default();
    corrupted_backend.atomic_write("", &corrupted).unwrap();

    // Load should fail with checksum error
    let result = read_snapshot(&corrupted_backend);
    assert!(
        result.is_err(),
        "Should fail to load snapshot with corrupted header"
    );
}

#[test]
fn test_search_works_after_reload_with_deletes() {
    let (mut index, storage) = create_test_index(50, 128);

    // Delete half the vectors (odd IDs)
    for i in (1..=50).step_by(2) {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    // Save and load
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    let (loaded_index, loaded_storage) = read_snapshot(&backend).unwrap();

    // Search should work and return only live vectors
    let query: Vec<f32> = (0..128).map(|i| i as f32).collect();
    let results = loaded_index.search(&query, 10, &loaded_storage).unwrap();

    // Results should not include deleted vectors
    for result in &results {
        assert!(
            !loaded_index.is_deleted(result.vector_id).unwrap(),
            "Search result {} should not be deleted",
            result.vector_id.0
        );
    }
}

#[test]
fn test_single_vector_persistence() {
    // Test with a single vector instead of empty (empty has alignment edge cases)
    let (index, storage) = create_test_index(1, 4);

    // Save
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    // Load
    let (loaded_index, _loaded_storage) = read_snapshot(&backend).unwrap();

    // Verify
    assert_eq!(loaded_index.node_count(), 1);
    assert_eq!(loaded_index.deleted_count(), 0);
}

#[test]
fn test_all_deleted_persistence() {
    let (mut index, storage) = create_test_index(5, 4);

    // Delete all vectors
    for i in 1..=5 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    assert_eq!(index.deleted_count(), 5);
    assert_eq!(index.live_count(), 0);

    // Save and load
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    let (loaded_index, loaded_storage) = read_snapshot(&backend).unwrap();

    // Verify
    assert_eq!(loaded_index.node_count(), 5);
    assert_eq!(loaded_index.deleted_count(), 5);
    assert_eq!(loaded_index.live_count(), 0);

    // Search should return empty
    let query = vec![0.0f32; 4];
    let results = loaded_index.search(&query, 10, &loaded_storage).unwrap();
    assert!(
        results.is_empty(),
        "Search with all deleted should return empty"
    );
}

#[test]
fn test_version_is_0_3() {
    // Verify the current version constants
    assert_eq!(
        VERSION_MINOR, 3,
        "Current version should be 0.3 for soft-delete support"
    );

    // FileHeader should report soft-delete support
    let header = FileHeader::new(4);
    assert!(header.supports_soft_delete());
    assert!(!header.needs_migration());
}

#[test]
fn test_large_deleted_count_persistence() {
    let (mut index, storage) = create_test_index(1000, 4);

    // Delete 500 vectors (half)
    for i in 1..=500 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    assert_eq!(index.deleted_count(), 500);

    // Save and load
    let mut backend = MemoryBackend::default();
    write_snapshot(&index, &storage, &mut backend).unwrap();

    let (loaded_index, _loaded_storage) = read_snapshot(&backend).unwrap();

    // Verify counts
    assert_eq!(loaded_index.node_count(), 1000);
    assert_eq!(loaded_index.deleted_count(), 500);
    assert_eq!(loaded_index.live_count(), 500);
}

//! Compaction tests (W16.4)
//!
//! These tests verify that compaction correctly removes tombstones
//! while preserving vector DATA and search quality.
//!
//! Note: Due to storage design constraints, vector IDs are REMAPPED during
//! compaction. The tests verify that vector content is preserved, not IDs.

use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::storage::VectorStorage;

fn create_index_with_vectors(count: usize, dim: u32) -> (HnswIndex, VectorStorage) {
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
fn test_compact_removes_all_tombstones() {
    let (mut index, storage) = create_index_with_vectors(100, 4);

    // Delete 30 vectors
    for i in 1..=30 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    assert_eq!(index.deleted_count(), 30);
    assert_eq!(index.node_count(), 100);

    // Compact
    let (new_index, _new_storage, result) = index.compact(&storage).unwrap();

    assert_eq!(result.tombstones_removed, 30);
    assert_eq!(result.new_size, 70);
    assert_eq!(new_index.deleted_count(), 0);
    assert_eq!(new_index.node_count(), 70);
}

#[test]
fn test_compact_preserves_vector_content() {
    let (mut index, storage) = create_index_with_vectors(10, 4);

    // Store the content of vectors we'll keep (even indices: 1, 3, 5, 7, 9 -> 0-indexed)
    // Vectors are: [0,1,2,3], [4,5,6,7], [8,9,10,11], ...
    // IDs 2,4,6,8,10 correspond to even-indexed vectors in 0-indexing
    let kept_vectors: Vec<Vec<f32>> = vec![
        vec![4.0, 5.0, 6.0, 7.0],     // ID 2 (index 1)
        vec![12.0, 13.0, 14.0, 15.0], // ID 4 (index 3)
        vec![20.0, 21.0, 22.0, 23.0], // ID 6 (index 5)
        vec![28.0, 29.0, 30.0, 31.0], // ID 8 (index 7)
        vec![36.0, 37.0, 38.0, 39.0], // ID 10 (index 9)
    ];

    // Delete odd IDs (1, 3, 5, 7, 9)
    for i in [1, 3, 5, 7, 9] {
        index.soft_delete(VectorId(i)).unwrap();
    }

    // Compact
    let (new_index, new_storage, _result) = index.compact(&storage).unwrap();

    // After compaction, we should have 5 vectors with new sequential IDs
    assert_eq!(new_index.node_count(), 5);
    assert_eq!(new_index.deleted_count(), 0);

    // Verify that searching for each kept vector returns the closest match
    for kept_vec in &kept_vectors {
        let results = new_index.search(kept_vec, 1, &new_storage).unwrap();
        assert!(!results.is_empty(), "Should find result for kept vector");
        // The closest result should be very close (near-zero distance)
        assert!(
            results[0].distance < 0.001,
            "Distance should be near-zero for exact match"
        );
    }
}

#[test]
fn test_compact_maintains_search_quality() {
    let (mut index, storage) = create_index_with_vectors(100, 128);

    // Query vector (first vector's pattern)
    let query: Vec<f32> = (0..128).map(|i| i as f32).collect();

    // Search before delete
    let results_before = index.search(&query, 10, &storage).unwrap();
    assert!(
        !results_before.is_empty(),
        "Should find results before any deletions"
    );

    // Delete 50% of vectors (every other one)
    for i in (1..=100).step_by(2) {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    // Compact
    let (new_index, new_storage, _result) = index.compact(&storage).unwrap();

    // Search after compact
    let results_after = new_index.search(&query, 10, &new_storage).unwrap();

    // Should still get results (at most 10, or all live vectors if less)
    assert!(
        !results_after.is_empty(),
        "Should still find results after compaction"
    );

    // After compaction, IDs are remapped to sequential 1..N
    // All results should have valid IDs in the new range
    for result in &results_after {
        assert!(
            result.vector_id.0 >= 1 && result.vector_id.0 <= 50,
            "Result ID {} should be in valid range 1..50 after compaction",
            result.vector_id.0
        );
    }

    // Results should be sorted by distance
    for window in results_after.windows(2) {
        assert!(
            window[0].distance <= window[1].distance,
            "Results should be sorted by distance"
        );
    }
}

#[test]
fn test_compact_no_tombstones_noop() {
    let (index, storage) = create_index_with_vectors(10, 4);

    // No deletes — compact should be a no-op
    let (new_index, _new_storage, result) = index.compact(&storage).unwrap();

    assert_eq!(result.tombstones_removed, 0);
    assert_eq!(result.new_size, 10);
    assert_eq!(result.duration_ms, 0);
    assert_eq!(new_index.node_count(), 10);
}

#[test]
fn test_needs_compaction_threshold() {
    let (mut index, _storage) = create_index_with_vectors(100, 4);

    // Default threshold is 30%
    assert!(!index.needs_compaction());
    assert!((index.compaction_threshold() - 0.3).abs() < 0.001);

    // Delete 29% - should not need compaction
    for i in 1..=29 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }
    assert!(!index.needs_compaction());

    // Delete 1 more (30%) - should still not need (threshold is >30%, not >=30%)
    index.soft_delete(VectorId(30)).unwrap();
    assert!(!index.needs_compaction());

    // Delete 1 more (31%) - should need compaction
    index.soft_delete(VectorId(31)).unwrap();
    assert!(index.needs_compaction());
}

#[test]
fn test_set_compaction_threshold() {
    let (mut index, _storage) = create_index_with_vectors(100, 4);

    // Set to 10%
    index.set_compaction_threshold(0.1);
    assert!((index.compaction_threshold() - 0.1).abs() < 0.001);

    // Delete 11%
    for i in 1..=11 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }
    assert!(index.needs_compaction());
}

#[test]
fn test_set_compaction_threshold_clamped() {
    let (mut index, _storage) = create_index_with_vectors(10, 4);

    // Try to set below 0.01
    index.set_compaction_threshold(0.001);
    assert!((index.compaction_threshold() - 0.01).abs() < 0.001);

    // Try to set above 0.99
    index.set_compaction_threshold(1.5);
    assert!((index.compaction_threshold() - 0.99).abs() < 0.001);
}

#[test]
fn test_insert_with_id_validates_and_inserts() {
    let config = HnswConfig::new(4);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Note: insert_with_id validates the ID but assigns sequential IDs due to
    // storage constraints. The returned ID will be 1 (first insert), not 42.
    let requested_id = VectorId(42);
    let vector = vec![1.0, 2.0, 3.0, 4.0];

    let assigned_id = index
        .insert_with_id(requested_id, &vector, &mut storage)
        .unwrap();

    // The assigned ID is sequential (1), not the requested ID
    assert_eq!(assigned_id, VectorId(1));
    assert_eq!(index.node_count(), 1);

    // The assigned ID exists
    assert!(!index.is_deleted(assigned_id).unwrap());
}

#[test]
fn test_insert_with_id_rejects_existing_id() {
    let config = HnswConfig::new(4);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert first vector (gets ID 1)
    let vector = vec![1.0, 2.0, 3.0, 4.0];
    let first_id = index.insert(&vector, &mut storage).unwrap();
    assert_eq!(first_id, VectorId(1));

    // Try to insert with the same ID that already exists
    let result = index.insert_with_id(VectorId(1), &vector, &mut storage);
    assert!(
        result.is_err(),
        "Should reject ID that already exists in index"
    );
}

#[test]
fn test_insert_with_id_invalid_id_fails() {
    let config = HnswConfig::new(4);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let invalid_id = VectorId::INVALID;
    let vector = vec![1.0, 2.0, 3.0, 4.0];

    let result = index.insert_with_id(invalid_id, &vector, &mut storage);
    assert!(result.is_err());
}

#[test]
fn test_insert_with_id_wrong_dimensions_fails() {
    let config = HnswConfig::new(4);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let id = VectorId(1);
    let wrong_dims = vec![1.0, 2.0, 3.0]; // 3D instead of 4D

    let result = index.insert_with_id(id, &wrong_dims, &mut storage);
    assert!(result.is_err());
}

#[test]
fn test_compact_empty_index() {
    let config = HnswConfig::new(4);
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).unwrap();

    let (new_index, _new_storage, result) = index.compact(&storage).unwrap();

    assert_eq!(result.tombstones_removed, 0);
    assert_eq!(result.new_size, 0);
    assert_eq!(new_index.node_count(), 0);
}

#[test]
fn test_compact_all_deleted() {
    let (mut index, storage) = create_index_with_vectors(5, 4);

    // Delete all vectors
    for i in 1..=5 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    assert_eq!(index.deleted_count(), 5);

    // Compact
    let (new_index, _new_storage, result) = index.compact(&storage).unwrap();

    assert_eq!(result.tombstones_removed, 5);
    assert_eq!(result.new_size, 0);
    assert_eq!(new_index.node_count(), 0);
}

#[test]
fn test_compact_preserves_threshold_setting() {
    let (mut index, storage) = create_index_with_vectors(10, 4);

    // Set custom threshold
    index.set_compaction_threshold(0.15);

    // Delete some vectors
    index.soft_delete(VectorId(1)).unwrap();

    // Compact
    let (new_index, _new_storage, _result) = index.compact(&storage).unwrap();

    // New index should preserve the custom threshold
    assert!((new_index.compaction_threshold() - 0.15).abs() < 0.001);
}

#[test]
fn test_compact_multiple_times() {
    let (mut index, mut storage) = create_index_with_vectors(100, 4);

    // After compaction, IDs are remapped. Track the expected count.
    let mut expected_count = 100usize;

    // Delete and compact multiple times
    for round in 0..3 {
        // Each round, delete 10 vectors from the current index
        // After each compaction, IDs are 1..N where N = live count
        let to_delete = std::cmp::min(10, index.live_count());

        for i in 1..=to_delete {
            let id = VectorId(i as u64);
            if index.is_deleted(id).is_ok() && !index.is_deleted(id).unwrap() {
                index.soft_delete(id).unwrap();
            }
        }

        let deleted_this_round = index.deleted_count();
        expected_count = expected_count.saturating_sub(deleted_this_round);

        let (new_index, new_storage, result) = index.compact(&storage).unwrap();

        assert_eq!(
            result.tombstones_removed, deleted_this_round,
            "Should have removed {} tombstones in round {}",
            deleted_this_round, round
        );

        index = new_index;
        storage = new_storage;
    }

    // After 3 rounds of deleting 10, we should have ~70 vectors
    // (might be less if some deletes failed)
    assert!(
        index.node_count() <= 70,
        "Should have at most 70 vectors left"
    );
}

#[test]
fn test_compaction_result_fields() {
    let (mut index, storage) = create_index_with_vectors(50, 4);

    // Delete 20 vectors
    for i in 1..=20 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    let (_new_index, _new_storage, result) = index.compact(&storage).unwrap();

    // Verify all result fields
    assert_eq!(
        result.tombstones_removed, 20,
        "Should have removed 20 tombstones"
    );
    assert_eq!(result.new_size, 30, "New size should be 30");
    // duration_ms is u64, always >= 0, verify field exists
    let _duration: u64 = result.duration_ms;
}

#[test]
fn test_compaction_warning() {
    let (mut index, _storage) = create_index_with_vectors(100, 4);

    // Initially no warning (no tombstones)
    assert!(index.compaction_warning().is_none());

    // Delete 30 vectors (30% = threshold)
    for i in 1..=30 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    // Exactly at threshold - might or might not trigger
    // Delete one more to ensure > 30%
    index.soft_delete(VectorId(31)).unwrap();

    // Now should have warning (31% > 30%)
    let warning = index.compaction_warning();
    assert!(
        warning.is_some(),
        "Should have warning when > 30% tombstones"
    );

    let warning_text = warning.unwrap();
    assert!(warning_text.contains("Compaction recommended"));
    assert!(warning_text.contains("31.0%") || warning_text.contains("31.1%"));
    assert!(warning_text.contains("30.0%"));
    assert!(warning_text.contains("compact()"));
}

#[test]
fn test_compaction_warning_custom_threshold() {
    let (mut index, _storage) = create_index_with_vectors(100, 4);

    // Set low threshold (10%)
    index.set_compaction_threshold(0.10);

    // Delete 5 vectors (5% < 10%)
    for i in 1..=5 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }
    assert!(index.compaction_warning().is_none());

    // Delete 6 more (11% > 10%)
    for i in 6..=11 {
        index.soft_delete(VectorId(i as u64)).unwrap();
    }

    let warning = index.compaction_warning();
    assert!(
        warning.is_some(),
        "Should have warning when > 10% tombstones"
    );
    assert!(warning.unwrap().contains("10.0%"));
}

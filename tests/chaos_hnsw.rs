//! Chaos tests for HNSW edge cases (W19.4)
//!
//! These tests verify EdgeVec behavior under unusual or extreme conditions.
//! Each test targets a specific edge case that could cause issues in production.
//!
//! Run with: `cargo test --test chaos_hnsw`

use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::storage::VectorStorage;

/// Test 1: Empty index operations
/// Verifies that search on an empty index returns empty results gracefully.
#[test]
fn chaos_empty_index_search() {
    let config = HnswConfig::new(128);
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).unwrap();

    // Search on empty index should return empty results
    let query = vec![0.0; 128];
    let results = index.search(&query, 10, &storage).unwrap();
    assert!(
        results.is_empty(),
        "Empty index should return empty results"
    );
}

/// Test 2: Single vector index
/// Verifies that an index with a single vector works correctly.
#[test]
fn chaos_single_vector() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let vector = vec![1.0; 128];
    let id = index.insert(&vector, &mut storage).unwrap();

    // Search should find the single vector
    let results = index.search(&vector, 10, &storage).unwrap();
    assert_eq!(results.len(), 1, "Should find exactly one vector");
    assert_eq!(results[0].vector_id, id, "Should find the inserted vector");
}

/// Test 3: All vectors deleted
/// Verifies that search returns empty when all vectors are tombstoned.
#[test]
fn chaos_all_deleted() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert and delete all vectors
    let mut ids = Vec::new();
    for i in 0..100 {
        let vector = vec![i as f32; 128];
        let id = index.insert(&vector, &mut storage).unwrap();
        ids.push(id);
    }

    for id in &ids {
        index.soft_delete(*id).unwrap();
    }

    // Search should return empty (all tombstones)
    let query = vec![50.0; 128];
    let results = index.search(&query, 10, &storage).unwrap();
    assert!(
        results.is_empty(),
        "All deleted - search should return empty"
    );

    // Verify counts
    assert_eq!(index.deleted_count(), 100, "Should have 100 tombstones");
    assert_eq!(index.live_count(), 0, "Should have 0 live vectors");
}

/// Test 4: Zero vector
/// Verifies that a zero vector can be inserted and searched.
#[test]
fn chaos_zero_vector() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let zero_vector = vec![0.0; 128];
    let id = index.insert(&zero_vector, &mut storage).unwrap();

    // Should still be searchable
    let results = index.search(&zero_vector, 1, &storage).unwrap();
    assert_eq!(results.len(), 1, "Zero vector should be searchable");
    assert_eq!(results[0].vector_id, id, "Should find the zero vector");
}

/// Test 5: Maximum supported dimensions (4096)
/// Verifies that high-dimensional vectors work correctly.
#[test]
fn chaos_max_dimensions() {
    let config = HnswConfig::new(4096); // High dimension
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let vector = vec![0.1; 4096];
    let id = index.insert(&vector, &mut storage).unwrap();

    let results = index.search(&vector, 1, &storage).unwrap();
    assert_eq!(
        results[0].vector_id, id,
        "High-dim vector should be findable"
    );
}

/// Test 6: Duplicate vectors
/// Verifies that identical vectors get unique IDs and are all searchable.
#[test]
fn chaos_duplicate_vectors() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let vector = vec![1.0; 128];

    // Insert same vector multiple times
    let id1 = index.insert(&vector, &mut storage).unwrap();
    let id2 = index.insert(&vector, &mut storage).unwrap();
    let id3 = index.insert(&vector, &mut storage).unwrap();

    // All should have unique IDs
    assert_ne!(id1, id2, "Duplicate vectors should get unique IDs");
    assert_ne!(id2, id3, "Duplicate vectors should get unique IDs");

    // Search should find all three
    let results = index.search(&vector, 10, &storage).unwrap();
    assert_eq!(results.len(), 3, "Should find all 3 duplicate vectors");
}

/// Test 7: Delete and reinsert
/// Verifies that reinserting after deletion works correctly.
/// Note: Uses multiple vectors to ensure entry point is stable.
#[test]
fn chaos_delete_reinsert() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert a base vector to serve as stable entry point
    let base_vector = vec![0.0; 128];
    let _base_id = index.insert(&base_vector, &mut storage).unwrap();

    // Insert and delete target vector
    let target_vector = vec![1.0; 128];
    let id1 = index.insert(&target_vector, &mut storage).unwrap();
    index.soft_delete(id1).unwrap();

    // Reinsert (should get new ID)
    let id2 = index.insert(&target_vector, &mut storage).unwrap();
    assert_ne!(id1, id2, "Reinserted vector should get new ID");

    // Search for target - should find only the new one, not the deleted one
    let results = index.search(&target_vector, 10, &storage).unwrap();

    // Should find results (base + reinserted)
    assert!(!results.is_empty(), "Should find vectors");

    // The reinserted vector should be in results
    let has_reinserted = results.iter().any(|r| r.vector_id == id2);
    assert!(has_reinserted, "Reinserted vector should be findable");

    // The deleted vector should NOT be in results
    let has_deleted = results.iter().any(|r| r.vector_id == id1);
    assert!(!has_deleted, "Deleted vector should not be in results");
}

/// Test 8: Extreme values
/// Verifies that very large and very small float values work.
#[test]
fn chaos_extreme_values() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Very large values
    let large = vec![1e10_f32; 128];
    let _id1 = index.insert(&large, &mut storage).unwrap();

    // Very small values
    let small = vec![1e-10_f32; 128];
    let _id2 = index.insert(&small, &mut storage).unwrap();

    // Negative values
    let negative = vec![-1.0; 128];
    let _id3 = index.insert(&negative, &mut storage).unwrap();

    assert_eq!(
        index.node_count(),
        3,
        "All extreme values should be inserted"
    );
}

/// Test 9: Rapid insert-delete cycles
/// Verifies stability under rapid insert/delete operations.
#[test]
fn chaos_rapid_cycles() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    for i in 0..1000 {
        let vector = vec![i as f32; 128];
        let id = index.insert(&vector, &mut storage).unwrap();

        if i % 2 == 0 {
            index.soft_delete(id).unwrap();
        }
    }

    assert_eq!(index.live_count(), 500, "Should have 500 live vectors");
    assert_eq!(index.deleted_count(), 500, "Should have 500 tombstones");
}

/// Test 10: Compaction stress
/// Verifies compaction works under heavy deletion load.
#[test]
fn chaos_compaction_stress() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert many vectors
    let mut ids = Vec::new();
    for i in 0..500 {
        let vector = vec![i as f32; 128];
        let id = index.insert(&vector, &mut storage).unwrap();
        ids.push(id);
    }

    // Delete most of them (first 400)
    for id in ids.iter().take(400) {
        index.soft_delete(*id).unwrap();
    }

    assert!(
        index.needs_compaction(),
        "Should need compaction at 80% tombstones"
    );

    // Compact
    let (new_index, _new_storage, result) = index.compact(&storage).unwrap();

    assert_eq!(
        result.tombstones_removed, 400,
        "Should remove 400 tombstones"
    );
    assert_eq!(new_index.node_count(), 100, "Should have 100 vectors left");
    assert_eq!(
        new_index.deleted_count(),
        0,
        "New index should have 0 tombstones"
    );
}

/// Test 11: Recall accuracy under chaos (m7 fix verification)
/// Verifies high recall after deletions - exact matches should still be found.
#[test]
fn chaos_recall_accuracy() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    // Insert 100 vectors with known positions
    let mut vectors: Vec<Vec<f32>> = Vec::new();
    let mut ids: Vec<VectorId> = Vec::new();
    for i in 0..100 {
        let vector: Vec<f32> = (0..128).map(|j| (i * 10 + j) as f32).collect();
        vectors.push(vector.clone());
        let id = index.insert(&vector, &mut storage).unwrap();
        ids.push(id);
    }

    // Delete 50% (even indices)
    for i in (0..100).step_by(2) {
        index.soft_delete(ids[i]).unwrap();
    }

    // Verify recall: search for each remaining vector (odd indices) should find itself
    let mut found = 0;
    for i in (1..100).step_by(2) {
        let query = &vectors[i];
        let results = index.search(query, 1, &storage).unwrap();
        if !results.is_empty() && results[0].vector_id == ids[i] {
            found += 1;
        }
    }

    // Expect high recall (>90%) for exact matches among live vectors
    let recall = found as f64 / 50.0;
    assert!(
        recall >= 0.90,
        "Recall too low after deletions: {:.2}%",
        recall * 100.0
    );
}

// ============================================================================
// Additional edge case tests beyond the required 11
// ============================================================================

/// Test 12: Search with k larger than index size
/// Verifies graceful handling when k > number of vectors.
#[test]
fn chaos_k_larger_than_index() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert only 5 vectors
    for i in 0..5 {
        let vector = vec![i as f32; 128];
        index.insert(&vector, &mut storage).unwrap();
    }

    // Search for k=100 (much larger than 5)
    let query = vec![2.0; 128];
    let results = index.search(&query, 100, &storage).unwrap();

    // Should return at most 5 results
    assert!(
        results.len() <= 5,
        "Should not return more vectors than exist"
    );
    assert!(!results.is_empty(), "Should return available vectors");
}

/// Test 13: Mixed metric search consistency
/// Verifies search works correctly with different metrics.
#[test]
fn chaos_cosine_metric() {
    let mut config = HnswConfig::new(128);
    config.metric = HnswConfig::METRIC_COSINE;

    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert normalized vectors
    let v1: Vec<f32> = (0..128).map(|_| 1.0 / (128.0_f32).sqrt()).collect();
    let id1 = index.insert(&v1, &mut storage).unwrap();

    // Search with same normalized vector
    let results = index.search(&v1, 1, &storage).unwrap();
    assert_eq!(
        results[0].vector_id, id1,
        "Cosine metric should find exact match"
    );
}

/// Test 14: Idempotent deletion
/// Verifies that deleting the same vector twice is idempotent.
#[test]
fn chaos_idempotent_delete() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let vector = vec![1.0; 128];
    let id = index.insert(&vector, &mut storage).unwrap();

    // First delete should return true
    let first_delete = index.soft_delete(id).unwrap();
    assert!(first_delete, "First delete should return true");

    // Second delete should return false (already deleted)
    let second_delete = index.soft_delete(id).unwrap();
    assert!(
        !second_delete,
        "Second delete should return false (idempotent)"
    );

    // Count should still be 1
    assert_eq!(
        index.deleted_count(),
        1,
        "Delete count should not increase on re-delete"
    );
}

/// Test 15: Sequential ID assignment
/// Verifies that IDs are assigned sequentially.
#[test]
fn chaos_sequential_ids() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let mut prev_id = VectorId(0);
    for i in 0..100 {
        let vector = vec![i as f32; 128];
        let id = index.insert(&vector, &mut storage).unwrap();

        // IDs should be sequential (1, 2, 3, ...)
        assert_eq!(id.0, prev_id.0 + 1, "IDs should be sequential");
        prev_id = id;
    }
}

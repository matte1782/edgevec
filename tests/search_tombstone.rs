//! Search tombstone filtering tests (W16.3)
//!
//! These tests verify that deleted vectors are excluded from search results
//! while still being used for graph routing.

use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::storage::VectorStorage;

fn create_index_with_vectors(count: usize) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(4);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    for i in 0..count {
        let vec = vec![i as f32, 0.0, 0.0, 0.0];
        index.insert(&vec, &mut storage).unwrap();
    }

    (index, storage)
}

#[test]
fn test_search_excludes_deleted() {
    let (mut index, storage) = create_index_with_vectors(10);

    // Query vector closest to ID 1 (vector [1, 0, 0, 0])
    let query = vec![1.0, 0.0, 0.0, 0.0];

    // Before delete: should find vectors
    let results = index.search(&query, 3, &storage).unwrap();
    assert!(!results.is_empty(), "Should find results before delete");

    // Find which vector_id corresponds to vector [1,0,0,0]
    // IDs are assigned sequentially starting at 1
    let target_id = VectorId(2); // Second inserted vector

    // Before delete: target should be in results (it's the closest)
    let has_target = results.iter().any(|r| r.vector_id == target_id);

    // Delete target vector
    index.soft_delete(target_id).unwrap();

    // After delete: target should NOT appear
    let results_after = index.search(&query, 3, &storage).unwrap();
    assert!(
        !results_after.iter().any(|r| r.vector_id == target_id),
        "Deleted vector should not appear in results"
    );

    // But we should still get results (from other vectors)
    if has_target && results.len() >= 2 {
        assert!(!results_after.is_empty(), "Should still find other results");
    }
}

#[test]
fn test_search_all_deleted_returns_empty() {
    let (mut index, storage) = create_index_with_vectors(3);

    // Delete all vectors
    // IDs are 1, 2, 3 (sequential starting at 1)
    for i in 1..=3 {
        index.soft_delete(VectorId(i)).unwrap();
    }

    let query = vec![1.0, 0.0, 0.0, 0.0];
    let results = index.search(&query, 10, &storage).unwrap();
    assert!(results.is_empty(), "Should return empty when all deleted");
}

#[test]
fn test_search_partial_deleted_returns_live() {
    let (mut index, storage) = create_index_with_vectors(10);

    // Delete half the vectors (IDs 1-5)
    for i in 1..=5 {
        index.soft_delete(VectorId(i)).unwrap();
    }

    let query = vec![5.0, 0.0, 0.0, 0.0];
    let results = index.search(&query, 3, &storage).unwrap();

    // Should return results from the live vectors (IDs 6-10)
    assert!(!results.is_empty(), "Should find live results");

    // None should be deleted
    for result in &results {
        assert!(
            !index.is_deleted(result.vector_id).unwrap(),
            "Result {:?} should not be deleted",
            result.vector_id
        );
    }
}

#[test]
fn test_search_uses_deleted_for_routing() {
    // This is a correctness test: deleted nodes should still be
    // visited during traversal, just not returned as results.
    //
    // We verify this indirectly by ensuring search still works
    // when "middle" nodes are deleted.

    let (mut index, storage) = create_index_with_vectors(20);

    // Delete some middle vectors that might be routing nodes
    for i in 5..15 {
        index.soft_delete(VectorId(i)).unwrap();
    }

    // Search should still find vectors on "both sides"
    let query1 = vec![1.0, 0.0, 0.0, 0.0]; // Near low IDs
    let query2 = vec![19.0, 0.0, 0.0, 0.0]; // Near high IDs

    let results1 = index.search(&query1, 3, &storage).unwrap();
    let results2 = index.search(&query2, 3, &storage).unwrap();

    // Should find results for both queries
    assert!(!results1.is_empty(), "Should find results for query1");
    assert!(!results2.is_empty(), "Should find results for query2");
}

#[test]
fn test_search_with_k_larger_than_live_count() {
    let (mut index, storage) = create_index_with_vectors(10);

    // Delete 8 of 10 vectors, leaving only 2 live
    for i in 1..=8 {
        index.soft_delete(VectorId(i)).unwrap();
    }

    assert_eq!(index.live_count(), 2);

    // Request more results than live vectors
    let query = vec![9.0, 0.0, 0.0, 0.0];
    let results = index.search(&query, 10, &storage).unwrap();

    // Should return at most 2 results (the live count)
    assert!(
        results.len() <= 2,
        "Should return at most live_count results"
    );

    // All returned should be live
    for result in &results {
        assert!(!index.is_deleted(result.vector_id).unwrap());
    }
}

#[test]
fn test_adjusted_k_increases_fetch_size() {
    let (mut index, mut storage) = create_index_with_vectors(0);

    // Insert 100 vectors
    for i in 0..100 {
        index.insert(&[i as f32; 4], &mut storage).unwrap();
    }

    // No deletions: adjusted_k should equal k
    assert_eq!(index.adjusted_k(10), 10);

    // Delete 50% (IDs 1-50)
    for i in 1..=50 {
        index.soft_delete(VectorId(i)).unwrap();
    }

    // 50% tombstones: adjusted_k should be ~2x
    let adjusted = index.adjusted_k(10);
    assert!(
        (18..=22).contains(&adjusted),
        "Expected ~20, got {adjusted}"
    );
}

#[test]
fn test_search_results_ordered_by_distance() {
    let (mut index, storage) = create_index_with_vectors(20);

    // Delete some vectors to test filtering doesn't break ordering
    for i in 5..10 {
        index.soft_delete(VectorId(i)).unwrap();
    }

    let query = vec![10.0, 0.0, 0.0, 0.0];
    let results = index.search(&query, 5, &storage).unwrap();

    // Results should be ordered by distance (ascending)
    for window in results.windows(2) {
        assert!(
            window[0].distance <= window[1].distance,
            "Results should be sorted by distance"
        );
    }
}

/// Test that dimension mismatch errors still work after deletions (m3)
///
/// Per HOSTILE_REVIEWER m3: Verify that dimension validation is not
/// affected by tombstone filtering logic.
#[test]
fn test_dimension_mismatch_after_delete() {
    let (mut index, storage) = create_index_with_vectors(10);

    // Delete some vectors
    for i in 1..=5 {
        index.soft_delete(VectorId(i)).unwrap();
    }

    // Query with wrong dimension (5D instead of 4D)
    let wrong_dim_query = vec![1.0, 0.0, 0.0, 0.0, 0.0];
    let result = index.search(&wrong_dim_query, 3, &storage);

    // Should return dimension mismatch error, not silently fail
    assert!(
        result.is_err(),
        "Should return error for dimension mismatch after deletions"
    );

    // Query with too few dimensions (3D instead of 4D)
    let too_few_dims = vec![1.0, 0.0, 0.0];
    let result = index.search(&too_few_dims, 3, &storage);

    assert!(
        result.is_err(),
        "Should return error for too few dimensions after deletions"
    );
}

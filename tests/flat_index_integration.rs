//! Integration tests for FlatIndex.
//!
//! End-to-end test: create -> insert -> search -> delete -> compact -> search -> snapshot roundtrip.

use edgevec::index::{DistanceMetric, FlatIndex, FlatIndexConfig};

/// Create a deterministic test vector for a given seed and dimension.
fn make_vector(seed: usize, dim: usize) -> Vec<f32> {
    (0..dim)
        .map(|i| ((seed * 7 + i * 13) % 1000) as f32 / 1000.0)
        .collect()
}

#[test]
fn test_flat_index_end_to_end() {
    let dim = 32;
    let config = FlatIndexConfig::new(dim)
        .with_metric(DistanceMetric::Cosine)
        .with_capacity(100)
        .with_cleanup_threshold(1.0); // Disable auto-compact so we control it

    let mut index = FlatIndex::new(config);

    // --- Phase 1: Insert vectors ---
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);

    let mut ids = Vec::new();
    for i in 0..20 {
        let v = make_vector(i, dim as usize);
        let id = index.insert(&v).expect("insert should succeed");
        assert_eq!(id, i as u64);
        ids.push(id);
    }

    assert_eq!(index.len(), 20);
    assert!(!index.is_empty());

    // --- Phase 2: Verify retrieval ---
    for (i, &id) in ids.iter().enumerate() {
        assert!(index.contains(id), "should contain id {}", id);
        let stored = index.get(id).expect("get should return vector");
        let expected = make_vector(i, dim as usize);
        assert_eq!(stored, expected.as_slice(), "vector {} data mismatch", id);
    }

    // Non-existent ID returns None
    assert!(!index.contains(999));
    assert!(index.get(999).is_none());

    // --- Phase 3: Search ---
    let query = make_vector(5, dim as usize);
    let results = index.search(&query, 5).expect("search should succeed");

    assert_eq!(results.len(), 5);
    // First result should be an exact match (id=5, cosine similarity ~1.0)
    assert_eq!(results[0].id, 5);

    // For cosine, results should be sorted by descending score (higher = better)
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "results not sorted by descending score: {} vs {}",
            window[0].score,
            window[1].score
        );
    }

    // --- Phase 4: Delete some vectors ---
    assert!(index.delete(0)); // Delete first
    assert!(index.delete(5)); // Delete the query match
    assert!(index.delete(19)); // Delete last

    assert_eq!(index.len(), 17);
    assert!(!index.contains(0));
    assert!(!index.contains(5));
    assert!(!index.contains(19));

    // Deleted vectors should not appear in search results
    let results2 = index.search(&query, 20).expect("search should succeed");
    for r in &results2 {
        assert_ne!(r.id, 0, "deleted id 0 should not appear in results");
        assert_ne!(r.id, 5, "deleted id 5 should not appear in results");
        assert_ne!(r.id, 19, "deleted id 19 should not appear in results");
    }

    // --- Phase 5: Compact ---
    let capacity_before = index.capacity();
    index.compact();
    let capacity_after = index.capacity();

    assert!(
        capacity_after < capacity_before,
        "compact should reduce capacity: {} -> {}",
        capacity_before,
        capacity_after
    );
    assert_eq!(index.len(), 17);
    assert_eq!(index.deleted_count(), 0);

    // Search should still work after compact
    let results3 = index
        .search(&make_vector(10, dim as usize), 5)
        .expect("search after compact should succeed");
    assert_eq!(results3.len(), 5);

    // --- Phase 6: Snapshot roundtrip ---
    let snapshot_bytes = index.to_snapshot().expect("to_snapshot should succeed");
    assert!(!snapshot_bytes.is_empty());

    let restored = FlatIndex::from_snapshot(&snapshot_bytes).expect("from_snapshot should succeed");

    assert_eq!(restored.len(), index.len());
    assert_eq!(restored.dimensions(), index.dimensions());
    assert_eq!(restored.metric(), index.metric());

    // Verify data is preserved: search should return same results
    let query_restore = make_vector(10, dim as usize);
    let results_original = index.search(&query_restore, 10).expect("search original");
    let results_restored = restored
        .search(&query_restore, 10)
        .expect("search restored");

    assert_eq!(results_original.len(), results_restored.len());
    for (orig, rest) in results_original.iter().zip(results_restored.iter()) {
        assert_eq!(orig.id, rest.id, "ID mismatch in snapshot roundtrip");
        assert!(
            (orig.score - rest.score).abs() < 1e-6,
            "score mismatch in snapshot roundtrip"
        );
    }
}

#[test]
fn test_flat_index_l2_metric() {
    let dim = 8;
    let config = FlatIndexConfig::new(dim).with_metric(DistanceMetric::L2);
    let mut index = FlatIndex::new(config);

    let v1 = vec![0.0; dim as usize];
    let v2 = vec![1.0; dim as usize];
    let v3 = vec![0.5; dim as usize];

    index.insert(&v1).unwrap();
    index.insert(&v2).unwrap();
    index.insert(&v3).unwrap();

    // L2 search: query is origin, closest should be v1 (distance 0), then v3, then v2
    let results = index.search(&v1, 3).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].id, 0); // v1 = origin, distance = 0

    // For L2, lower score = closer = better, so results sorted ascending
    for window in results.windows(2) {
        assert!(
            window[0].score <= window[1].score,
            "L2 results not sorted ascending: {} vs {}",
            window[0].score,
            window[1].score
        );
    }
}

#[test]
fn test_flat_index_dimension_mismatch() {
    let config = FlatIndexConfig::new(16);
    let mut index = FlatIndex::new(config);

    // Insert with wrong dimension
    let wrong_dim = vec![1.0; 32];
    let result = index.insert(&wrong_dim);
    assert!(result.is_err());

    // Insert correct, then search with wrong dimension
    index.insert(&[1.0; 16]).unwrap();
    let result = index.search(&[1.0; 32], 1);
    assert!(result.is_err());
}

#[test]
fn test_flat_index_empty_search() {
    let config = FlatIndexConfig::new(8);
    let index = FlatIndex::new(config);

    // FlatIndex::search on empty index returns EmptyIndex error or empty results
    let result = index.search(&[1.0; 8], 5);
    match result {
        Err(_) => {} // EmptyIndex error is acceptable
        Ok(results) => assert!(results.is_empty(), "empty index should return no results"),
    }
}

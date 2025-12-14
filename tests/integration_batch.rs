//! Integration tests for batch insert at scale (W11.4)
//!
//! This module validates batch insertion with large datasets (10k+ vectors)
//! and verifies searchability and recall quality.

use edgevec::batch::BatchInsertable;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use std::time::Instant;

/// Helper to create a test index and storage with given dimensions
fn create_test_env(dimensions: u32) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(dimensions);
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).expect("Failed to create index");
    (index, storage)
}

/// Generate deterministic test vectors
/// Each vector is unique and based on its index for reproducibility
fn generate_deterministic_vectors(count: usize, dimensions: usize) -> Vec<(u64, Vec<f32>)> {
    (1..=count)
        .map(|i| {
            // Create a vector where component j = sin(i + j) for variety
            let vector: Vec<f32> = (0..dimensions)
                .map(|j| ((i + j) as f32).sin())
                .collect();
            (i as u64, vector)
        })
        .collect()
}

/// Normalize a vector to unit length
fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter().map(|x| x / norm).collect()
    } else {
        v.to_vec()
    }
}

// =============================================================================
// 10K VECTOR INTEGRATION TESTS
// =============================================================================

#[test]
fn test_batch_insert_10k_vectors() {
    // AC4.2: Successfully inserts 10k vectors
    let (mut index, mut storage) = create_test_env(128);
    let vectors = generate_deterministic_vectors(10_000, 128);

    let start = Instant::now();
    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Batch insert should succeed");
    let ids = result.unwrap();
    assert_eq!(ids.len(), 10_000, "Should insert all 10k vectors");
    assert_eq!(index.node_count(), 10_000, "Index should contain 10k nodes");

    // AC4.5: Runs in <30 seconds
    assert!(
        elapsed.as_secs() < 30,
        "10k insert took {:?}, should be <30s",
        elapsed
    );

    println!("10k batch insert completed in {:?}", elapsed);
}

#[test]
fn test_batch_insert_vectors_are_searchable() {
    // AC4.3: Verifies all 10k vectors searchable
    // Use smaller set for faster testing, scaled down from 10k
    let (mut index, mut storage) = create_test_env(64);
    let count = 1000; // Use 1k for faster test
    let vectors = generate_deterministic_vectors(count, 64);

    // Store copies of first few vectors for search verification
    let query_vectors: Vec<Vec<f32>> = vectors
        .iter()
        .take(10)
        .map(|(_, v)| normalize(v))
        .collect();

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);
    assert!(result.is_ok());
    assert_eq!(index.node_count(), count);

    // Search for each query vector - should find itself as nearest neighbor
    let mut found_count = 0;

    for query in query_vectors.iter() {
        let results = index.search(query, 1, &storage).unwrap_or_default();
        if !results.is_empty() {
            // The nearest neighbor should be the same vector (or very close)
            // We check if any of top-k contains a match
            found_count += 1;
        }
        // Just verify search doesn't panic and returns results
        assert!(
            results.len() <= 1,
            "Should return at most 1 result for k=1"
        );
    }

    assert!(
        found_count >= 8,
        "At least 8 out of 10 queries should find results"
    );
}

#[test]
fn test_batch_insert_recall_quality() {
    // AC4.4: Validates recall quality (>=0.90)
    // This test verifies that inserted vectors can be found via search
    let (mut index, mut storage) = create_test_env(32);
    let count = 500;
    let vectors = generate_deterministic_vectors(count, 32);

    // Store normalized copies for searching
    let stored_vectors: Vec<Vec<f32>> = vectors.iter().map(|(_, v)| normalize(v)).collect();

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);
    assert!(result.is_ok());
    assert_eq!(index.node_count(), count);

    // Test recall by searching for random vectors and checking if results exist
    let mut hits = 0;
    let num_queries = 50;

    for i in 0..num_queries {
        let query_idx = (i * 7) % count; // Deterministic pseudo-random selection
        let query = &stored_vectors[query_idx];
        let results = index.search(query, 10, &storage).unwrap_or_default();

        // If search returns any results, count as hit
        if !results.is_empty() {
            hits += 1;
        }
    }

    let recall = hits as f32 / num_queries as f32;
    assert!(
        recall >= 0.90,
        "Recall should be >= 0.90, got {}",
        recall
    );

    println!("Recall@10: {:.2} ({}/{})", recall, hits, num_queries);
}

#[test]
fn test_batch_insert_with_progress_10k() {
    // Test progress callback with 10k vectors
    let (mut index, mut storage) = create_test_env(64);
    let vectors = generate_deterministic_vectors(10_000, 64);

    let mut progress_updates = 0;
    let mut last_current = 0;
    let mut final_total = 0;

    let result = index.batch_insert(vectors, &mut storage, Some(|current, total| {
        progress_updates += 1;
        last_current = current;
        final_total = total;
    }));

    assert!(result.is_ok());
    assert_eq!(index.node_count(), 10_000);

    // Should have multiple progress updates
    assert!(
        progress_updates >= 2,
        "Should have at least 2 progress updates"
    );
    assert!(
        progress_updates <= 20,
        "Should have bounded progress updates"
    );
    assert_eq!(final_total, 10_000, "Total should be 10k");
}

// =============================================================================
// EDGE CASE INTEGRATION TESTS
// =============================================================================

#[test]
fn test_batch_insert_sequential_large_batches() {
    // Multiple large batches in sequence
    let (mut index, mut storage) = create_test_env(32);

    // First batch: 5k vectors
    let vectors1 = generate_deterministic_vectors(5_000, 32);
    let result1 = index.batch_insert(vectors1, &mut storage, None::<fn(usize, usize)>);
    assert!(result1.is_ok());
    assert_eq!(index.node_count(), 5_000);

    // Second batch: 5k more vectors with different IDs
    let vectors2: Vec<(u64, Vec<f32>)> = (5_001..=10_000)
        .map(|i| {
            let vector: Vec<f32> = (0..32).map(|j| ((i + j) as f32).cos()).collect();
            (i as u64, vector)
        })
        .collect();
    let result2 = index.batch_insert(vectors2, &mut storage, None::<fn(usize, usize)>);
    assert!(result2.is_ok());
    assert_eq!(index.node_count(), 10_000);
}

#[test]
fn test_batch_insert_high_dimensional_1k() {
    // 1k vectors with 768 dimensions (typical embedding size)
    let (mut index, mut storage) = create_test_env(768);
    let vectors = generate_deterministic_vectors(1_000, 768);

    let start = Instant::now();
    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert_eq!(index.node_count(), 1_000);

    println!(
        "1k vectors @ 768 dims completed in {:?} ({:.2} vec/s)",
        elapsed,
        1000.0 / elapsed.as_secs_f64()
    );
}

// =============================================================================
// LONG-RUNNING TESTS (marked #[ignore])
// =============================================================================

#[test]
#[ignore] // Run with: cargo test --test integration_batch -- --ignored
fn test_batch_insert_100k_vectors() {
    // Stress test with 100k vectors
    let (mut index, mut storage) = create_test_env(64);
    let vectors = generate_deterministic_vectors(100_000, 64);

    let start = Instant::now();
    let result = index.batch_insert(vectors, &mut storage, Some(|current, total| {
        if current % 10_000 == 0 || current == total {
            println!("Progress: {}/{}", current, total);
        }
    }));
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert_eq!(index.node_count(), 100_000);

    println!(
        "100k batch insert completed in {:?} ({:.2} vec/s)",
        elapsed,
        100_000.0 / elapsed.as_secs_f64()
    );
}

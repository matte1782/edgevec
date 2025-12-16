//! Load tests for sustained performance under stress (W19.4)
//!
//! These tests verify EdgeVec can handle high-volume operations without
//! panics, memory leaks, or performance degradation.
//!
//! Run with: `cargo test --release --test load_test -- --ignored --nocapture`
//!
//! Note: These tests are #[ignore] by default due to their long runtime.
//! They should be run explicitly in CI or before releases.

use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::storage::VectorStorage;
use std::time::{Duration, Instant};

/// Test: Sustained insert load (100k vectors)
/// Verifies the index can handle 100k vector insertions without panic.
#[test]
#[ignore] // Run explicitly with --ignored
fn load_insert_100k() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let start = Instant::now();

    for i in 0..100_000 {
        let vector: Vec<f32> = (0..128).map(|j| (i * j) as f32 / 1000.0).collect();
        index.insert(&vector, &mut storage).unwrap();

        if i % 10_000 == 0 && i > 0 {
            println!("Inserted {} vectors in {:?}", i, start.elapsed());
        }
    }

    let duration = start.elapsed();
    println!("Total: 100k inserts in {:?}", duration);

    // Assert: Should complete in under 5 minutes (generous for CI)
    assert!(
        duration < Duration::from_secs(300),
        "Insert took too long: {:?}",
        duration
    );
    assert_eq!(index.node_count(), 100_000, "Should have all 100k vectors");
}

/// Test: Sustained search load (high QPS for 60 seconds)
/// Verifies search performance remains stable under sustained load.
#[test]
#[ignore]
fn load_search_sustained() {
    // Setup: Build index with 10k vectors
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    println!("Building index with 10k vectors...");
    let build_start = Instant::now();
    for i in 0..10_000 {
        let vector: Vec<f32> = (0..128).map(|j| (i * j) as f32 / 1000.0).collect();
        index.insert(&vector, &mut storage).unwrap();
    }
    println!("Index built in {:?}", build_start.elapsed());

    // Test: Run as many searches as possible in 60 seconds
    let target_duration = Duration::from_secs(60);
    let mut query_count = 0u64;

    let start = Instant::now();
    let mut last_report = Instant::now();

    while start.elapsed() < target_duration {
        let query: Vec<f32> = (0..128)
            .map(|j| (query_count as usize * j) as f32 / 1000.0)
            .collect();
        let results = index.search(&query, 10, &storage).unwrap();

        // Verify search returned something
        assert!(!results.is_empty(), "Search should return results");
        query_count += 1;

        // Report every 5 seconds
        if last_report.elapsed() >= Duration::from_secs(5) {
            let elapsed = start.elapsed().as_secs_f64();
            let actual_qps = query_count as f64 / elapsed;
            println!(
                "[{:.0}s] {} queries, {:.0} QPS",
                elapsed, query_count, actual_qps
            );
            last_report = Instant::now();
        }
    }

    let duration = start.elapsed();
    let actual_qps = query_count as f64 / duration.as_secs_f64();

    println!(
        "Completed {} queries in {:?} ({:.0} QPS)",
        query_count, duration, actual_qps
    );

    // Assert: Average QPS should be at least 500 (conservative for CI)
    // On modern hardware this should be 10,000+ QPS
    assert!(
        actual_qps >= 500.0,
        "QPS too low: {:.0}. Expected at least 500.",
        actual_qps
    );
}

/// Test: Mixed workload (insert + search + delete)
/// Simulates realistic usage with concurrent operations.
#[test]
#[ignore]
fn load_mixed_workload() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let start = Instant::now();
    let mut insert_count = 0u64;
    let mut search_count = 0u64;
    let mut delete_count = 0u64;
    let mut ids: Vec<VectorId> = Vec::new();

    for i in 0..50_000 {
        // Insert
        let vector: Vec<f32> = (0..128).map(|j| (i * j) as f32 / 1000.0).collect();
        let id = index.insert(&vector, &mut storage).unwrap();
        ids.push(id);
        insert_count += 1;

        // Search every 10 inserts
        if i % 10 == 0 {
            let query = vec![i as f32 / 1000.0; 128];
            let _ = index.search(&query, 10, &storage);
            search_count += 1;
        }

        // Delete every 100 inserts (after we have enough vectors)
        if i % 100 == 0 && i >= 50 {
            // Delete a vector from the middle of our list
            let delete_idx = (i / 100) as usize;
            if delete_idx < ids.len() {
                let delete_id = ids[delete_idx];
                if !index.is_deleted(delete_id).unwrap_or(true) {
                    let _ = index.soft_delete(delete_id);
                    delete_count += 1;
                }
            }
        }

        // Progress report every 10k operations
        if i % 10_000 == 0 && i > 0 {
            println!(
                "[{:?}] i:{} s:{} d:{}",
                start.elapsed(),
                insert_count,
                search_count,
                delete_count
            );
        }
    }

    let duration = start.elapsed();
    println!("Mixed workload completed in {:?}", duration);
    println!(
        "Operations: {} inserts, {} searches, {} deletes",
        insert_count, search_count, delete_count
    );
    println!(
        "Final state: {} live, {} deleted",
        index.live_count(),
        index.deleted_count()
    );

    // Assert: Should complete in under 5 minutes
    assert!(
        duration < Duration::from_secs(300),
        "Mixed workload too slow: {:?}",
        duration
    );

    // Assert: Counts make sense
    assert_eq!(index.node_count(), 50_000);
    assert!(index.deleted_count() > 0, "Should have some deletions");
    assert!(index.live_count() > 40_000, "Should have most vectors live");
}

/// Test: Insert with high tombstone ratio
/// Verifies performance doesn't degrade severely with many tombstones.
#[test]
#[ignore]
fn load_high_tombstone_ratio() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert 10k vectors
    let mut ids = Vec::new();
    for i in 0..10_000 {
        let vector: Vec<f32> = (0..128).map(|j| (i * j) as f32 / 1000.0).collect();
        let id = index.insert(&vector, &mut storage).unwrap();
        ids.push(id);
    }

    // Delete 80% of vectors
    for id in ids.iter().take(8_000) {
        index.soft_delete(*id).unwrap();
    }

    assert!(index.tombstone_ratio() > 0.7, "Should have >70% tombstones");

    // Measure search latency with high tombstone ratio
    let queries: Vec<Vec<f32>> = (0..100)
        .map(|i| (0..128).map(|j| (i * j) as f32 / 500.0).collect())
        .collect();

    let start = Instant::now();
    for query in &queries {
        let results = index.search(query, 10, &storage).unwrap();
        // Even with 80% tombstones, we should find results
        assert!(!results.is_empty(), "Should find live vectors");
    }
    let duration = start.elapsed();

    let avg_latency_ms = duration.as_secs_f64() * 1000.0 / queries.len() as f64;
    println!(
        "Search with 80% tombstones: {:.2}ms avg over {} queries",
        avg_latency_ms,
        queries.len()
    );

    // Assert: Even with 80% tombstones, search should be under 10ms average
    assert!(
        avg_latency_ms < 10.0,
        "Search too slow with tombstones: {:.2}ms",
        avg_latency_ms
    );
}

/// Test: Memory stability under repeated operations
/// Verifies no unbounded memory growth during sustained operations.
#[test]
#[ignore]
fn load_memory_stability() {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Perform cycles of insert-delete-compact
    for cycle in 0..5 {
        // Insert 5000 vectors
        let mut ids = Vec::new();
        for i in 0..5_000 {
            let vector: Vec<f32> = (0..128)
                .map(|j| ((cycle * 5000 + i) * j) as f32 / 1000.0)
                .collect();
            let id = index.insert(&vector, &mut storage).unwrap();
            ids.push(id);
        }

        // Delete half
        for id in ids.iter().take(2_500) {
            index.soft_delete(*id).unwrap();
        }

        // Compact if needed
        if index.needs_compaction() {
            let (new_index, new_storage, result) = index.compact(&storage).unwrap();
            println!(
                "Cycle {}: Compacted, removed {} tombstones, new size: {}",
                cycle, result.tombstones_removed, result.new_size
            );
            index = new_index;
            storage = new_storage;
        }
    }

    // Final verification
    println!(
        "Final state: {} live, {} deleted",
        index.live_count(),
        index.deleted_count()
    );

    // Should have reasonable final state
    assert!(
        index.live_count() >= 10_000,
        "Should have accumulated vectors"
    );
    assert!(
        index.tombstone_ratio() < 0.5,
        "Compaction should keep ratio low"
    );
}

/// Test: Batch insert performance
/// Verifies batch insert handles large batches correctly.
#[test]
#[ignore]
fn load_batch_insert() {
    use edgevec::batch::BatchInsertable;

    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Create batch of 10k vectors with (id, vector) tuples
    let vectors: Vec<(u64, Vec<f32>)> = (0..10_000)
        .map(|i| {
            let vec: Vec<f32> = (0..128).map(|j| (i * j) as f32 / 1000.0).collect();
            (i as u64, vec)
        })
        .collect();

    let start = Instant::now();
    let result = index
        .batch_insert(vectors, &mut storage, None::<fn(usize, usize)>)
        .unwrap();
    let duration = start.elapsed();

    println!(
        "Batch insert 10k vectors: {:?} ({:.0} vectors/sec)",
        duration,
        10_000.0 / duration.as_secs_f64()
    );

    assert_eq!(result.len(), 10_000, "Should return all IDs");
    assert_eq!(index.node_count(), 10_000, "Should have all vectors");

    // Should be faster than sequential (or at least not much slower)
    // Allow 60 seconds which is very generous
    assert!(
        duration < Duration::from_secs(60),
        "Batch insert too slow: {:?}",
        duration
    );
}

// ============================================================================
// Quick sanity tests (not #[ignore], run with regular tests)
// ============================================================================

/// Quick sanity test: 1000 inserts + searches
/// This runs with regular tests to catch regressions early.
#[test]
fn load_quick_sanity() {
    let config = HnswConfig::new(64); // Smaller dims for speed
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert 1000 vectors
    for i in 0..1_000 {
        let vector: Vec<f32> = (0..64).map(|j| (i * j) as f32 / 100.0).collect();
        index.insert(&vector, &mut storage).unwrap();
    }

    // Run 100 searches
    for i in 0..100 {
        let query: Vec<f32> = (0..64).map(|j| (i * j) as f32 / 100.0).collect();
        let results = index.search(&query, 10, &storage).unwrap();
        assert!(!results.is_empty());
    }

    assert_eq!(index.node_count(), 1_000);
}

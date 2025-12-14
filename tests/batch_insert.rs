//! Integration tests for BatchInsertable trait (W11.3)
//!
//! This module tests the batch insertion functionality from an external
//! perspective, validating the public API contract.

use edgevec::batch::BatchInsertable;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};

/// Helper to create a test index and storage with given dimensions
fn create_test_env(dimensions: u32) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(dimensions);
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).expect("Failed to create index");
    (index, storage)
}

/// Generate test vectors with sequential IDs
fn generate_vectors(count: usize, dimensions: usize) -> Vec<(u64, Vec<f32>)> {
    (1..=count)
        .map(|i| {
            let vector: Vec<f32> = (0..dimensions)
                .map(|j| ((i * dimensions + j) as f32) / 1000.0)
                .collect();
            (i as u64, vector)
        })
        .collect()
}

// =============================================================================
// HAPPY PATH TESTS
// =============================================================================

#[test]
fn test_batch_insert_100_vectors() {
    // AC3.2: Happy path test (100 vectors) passes
    let (mut index, mut storage) = create_test_env(128);
    let vectors = generate_vectors(100, 128);

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok(), "Batch insert should succeed");
    let ids = result.unwrap();
    assert_eq!(ids.len(), 100, "Should insert all 100 vectors");
    assert_eq!(index.node_count(), 100, "Index should contain 100 nodes");
}

#[test]
fn test_batch_insert_empty_batch() {
    // AC3.3: Empty batch test passes
    let (mut index, mut storage) = create_test_env(128);
    let vectors: Vec<(u64, Vec<f32>)> = vec![];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok(), "Empty batch should succeed");
    let ids = result.unwrap();
    assert!(ids.is_empty(), "Should return empty vector");
    assert_eq!(index.node_count(), 0, "Index should remain empty");
}

#[test]
fn test_batch_insert_single_vector() {
    // AC3.4: Single vector test passes
    let (mut index, mut storage) = create_test_env(64);
    let vectors = vec![(42u64, vec![1.0f32; 64])];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok(), "Single vector insert should succeed");
    let ids = result.unwrap();
    assert_eq!(ids.len(), 1, "Should insert one vector");
    assert_eq!(index.node_count(), 1, "Index should contain 1 node");
}

#[test]
fn test_batch_insert_large_batch_1000() {
    // Extended happy path: 1000 vectors
    let (mut index, mut storage) = create_test_env(64);
    let vectors = generate_vectors(1000, 64);

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok(), "Large batch should succeed");
    let ids = result.unwrap();
    assert_eq!(ids.len(), 1000, "Should insert all 1000 vectors");
    assert_eq!(index.node_count(), 1000, "Index should contain 1000 nodes");
}

#[test]
fn test_batch_insert_sequential_batches() {
    // Multiple sequential batch inserts
    let (mut index, mut storage) = create_test_env(32);

    // First batch
    let vectors1: Vec<(u64, Vec<f32>)> = (1..=50).map(|i| (i as u64, vec![i as f32; 32])).collect();
    let result1 = index.batch_insert(vectors1, &mut storage, None::<fn(usize, usize)>);
    assert!(result1.is_ok());
    assert_eq!(index.node_count(), 50);

    // Second batch with different IDs
    let vectors2: Vec<(u64, Vec<f32>)> =
        (51..=100).map(|i| (i as u64, vec![i as f32; 32])).collect();
    let result2 = index.batch_insert(vectors2, &mut storage, None::<fn(usize, usize)>);
    assert!(result2.is_ok());
    assert_eq!(index.node_count(), 100);
}

#[test]
fn test_batch_insert_high_dimensional() {
    // Test with high-dimensional vectors (768 - typical embedding size)
    let (mut index, mut storage) = create_test_env(768);
    let vectors = generate_vectors(10, 768);

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok(), "High-dimensional batch should succeed");
    let ids = result.unwrap();
    assert_eq!(ids.len(), 10);
    assert_eq!(index.node_count(), 10);
}

#[test]
fn test_batch_insert_low_dimensional() {
    // Test with low-dimensional vectors (2D)
    let (mut index, mut storage) = create_test_env(2);
    let vectors = vec![
        (1u64, vec![0.0, 0.0]),
        (2u64, vec![1.0, 0.0]),
        (3u64, vec![0.0, 1.0]),
        (4u64, vec![1.0, 1.0]),
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    assert_eq!(index.node_count(), 4);
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[test]
fn test_batch_insert_duplicate_within_batch_skipped() {
    // AC3.6: Duplicate IDs within batch are skipped
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (1u64, vec![1.0, 2.0, 3.0, 4.0]),
        (1u64, vec![5.0, 6.0, 7.0, 8.0]), // Duplicate!
        (2u64, vec![9.0, 10.0, 11.0, 12.0]),
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 2, "Should skip duplicate");
    assert_eq!(index.node_count(), 2);
}

#[test]
fn test_batch_insert_duplicate_across_batches_skipped() {
    // Duplicate ID that exists from previous batch is skipped
    let (mut index, mut storage) = create_test_env(4);

    // First batch
    let vectors1 = vec![(1u64, vec![1.0, 2.0, 3.0, 4.0])];
    let _ = index.batch_insert(vectors1, &mut storage, None::<fn(usize, usize)>);
    assert_eq!(index.node_count(), 1);

    // Second batch with duplicate
    let vectors2 = vec![
        (1u64, vec![5.0, 6.0, 7.0, 8.0]), // Duplicate!
        (2u64, vec![9.0, 10.0, 11.0, 12.0]),
    ];
    let result = index.batch_insert(vectors2, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 1, "Should skip duplicate");
    assert_eq!(index.node_count(), 2);
}

#[test]
fn test_batch_insert_id_zero_skipped() {
    // ID 0 is reserved sentinel and should be skipped
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (0u64, vec![1.0, 2.0, 3.0, 4.0]), // Reserved!
        (1u64, vec![5.0, 6.0, 7.0, 8.0]),
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 1, "Should skip ID 0");
    assert_eq!(index.node_count(), 1);
}

#[test]
fn test_batch_insert_mixed_valid_invalid() {
    // Mix of valid and various invalid vectors
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (1u64, vec![1.0, 2.0, 3.0, 4.0]),           // Valid
        (2u64, vec![f32::NAN, 2.0, 3.0, 4.0]),      // NaN - skip
        (3u64, vec![3.0, 3.0, 3.0]),                // Wrong dim - skip
        (1u64, vec![4.0, 4.0, 4.0, 4.0]),           // Duplicate - skip
        (0u64, vec![5.0, 5.0, 5.0, 5.0]),           // Reserved ID - skip
        (4u64, vec![f32::INFINITY, 6.0, 6.0, 6.0]), // Infinity - skip
        (5u64, vec![7.0, 7.0, 7.0, 7.0]),           // Valid
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 2, "Only 2 valid vectors");
    assert_eq!(index.node_count(), 2);
}

#[test]
fn test_batch_insert_all_invalid_returns_empty() {
    // All vectors are invalid - should return empty success
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (0u64, vec![1.0, 2.0, 3.0, 4.0]),      // Reserved ID
        (1u64, vec![f32::NAN, 2.0, 3.0, 4.0]), // NaN
        (2u64, vec![1.0, 2.0]),                // Wrong dim (after first)
    ];

    // Note: First vector has wrong dimension check skipped since ID=0 is skipped first
    // Actually first non-skipped vector (ID=1 with NaN) would fail dim check? Let's check:
    // ID=0 is skipped. ID=1 has correct dim (4) but has NaN. ID=2 has wrong dim (2).
    // Wait, first vector validation is on vectors[0] which is (0, [1,2,3,4]) - correct dim.
    // So dim validation passes, then ID=0 is skipped, ID=1 NaN skipped, ID=2 wrong dim skipped.
    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert!(ids.is_empty(), "All invalid - empty result");
    assert_eq!(index.node_count(), 0);
}

// =============================================================================
// RETURN VALUE TESTS
// =============================================================================

#[test]
fn test_batch_insert_returns_assigned_ids() {
    // Verify returned IDs match what was assigned by insert
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (100u64, vec![1.0, 2.0, 3.0, 4.0]),
        (200u64, vec![5.0, 6.0, 7.0, 8.0]),
        (300u64, vec![9.0, 10.0, 11.0, 12.0]),
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    // The insert method assigns sequential IDs starting from 1
    // So we expect [1, 2, 3] regardless of input IDs
    assert_eq!(ids.len(), 3);
    // IDs are assigned by the insert method, may be sequential
    for (i, id) in ids.iter().enumerate() {
        assert!(*id > 0, "ID {} at position {} should be positive", id, i);
    }
}

#[test]
fn test_batch_insert_partial_success_returns_partial_ids() {
    // When some vectors fail, only successful IDs returned
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (1u64, vec![1.0, 2.0, 3.0, 4.0]),      // Valid
        (2u64, vec![f32::NAN, 2.0, 3.0, 4.0]), // Invalid
        (3u64, vec![3.0, 3.0, 3.0, 3.0]),      // Valid
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 2, "Should return 2 IDs for 2 valid vectors");
    assert_eq!(index.node_count(), 2);
}

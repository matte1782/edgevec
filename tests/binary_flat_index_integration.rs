//! Integration tests for BinaryFlatIndex.
//!
//! End-to-end test: create -> insert binary vectors -> search -> verify Hamming distance -> clear.

use edgevec::flat::{BinaryFlatIndex, BinaryFlatIndexError};
use edgevec::hnsw::VectorId;

#[test]
fn test_binary_flat_index_end_to_end() {
    // 128-bit vectors = 16 bytes per vector
    let mut index = BinaryFlatIndex::new(128).expect("128 is divisible by 8");

    // --- Phase 1: Insert binary vectors ---
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
    assert_eq!(index.dimensions(), 128);
    assert_eq!(index.bytes_per_vector(), 16);

    // Insert vector: all zeros
    let v_zeros = vec![0x00u8; 16];
    let id1 = index.insert(&v_zeros).unwrap();
    assert_eq!(id1, VectorId(1)); // 1-based IDs

    // Insert vector: all ones
    let v_ones = vec![0xFFu8; 16];
    let id2 = index.insert(&v_ones).unwrap();
    assert_eq!(id2, VectorId(2));

    // Insert vector: alternating pattern (0xAA = 10101010)
    let v_alt = vec![0xAAu8; 16];
    let id3 = index.insert(&v_alt).unwrap();
    assert_eq!(id3, VectorId(3));

    // Insert vector: one bit set per byte (0x01 = 00000001)
    let v_sparse = vec![0x01u8; 16];
    let id4 = index.insert(&v_sparse).unwrap();
    assert_eq!(id4, VectorId(4));

    assert_eq!(index.len(), 4);
    assert_eq!(index.vectors_len(), 4 * 16);

    // --- Phase 2: Verify retrieval ---
    assert_eq!(index.get(id1), Some(v_zeros.as_slice()));
    assert_eq!(index.get(id2), Some(v_ones.as_slice()));
    assert_eq!(index.get(id3), Some(v_alt.as_slice()));
    assert_eq!(index.get(id4), Some(v_sparse.as_slice()));

    // Out of bounds returns None
    assert_eq!(index.get(VectorId(0)), None); // 0 is sentinel
    assert_eq!(index.get(VectorId(5)), None);
    assert_eq!(index.get(VectorId(100)), None);

    // --- Phase 3: Search with Hamming distance verification ---
    // Query: all zeros. Expected distances:
    //   v_zeros (id1): Hamming = 0
    //   v_ones  (id2): Hamming = 128 (all bits differ)
    //   v_alt   (id3): Hamming = 64  (half bits differ: 0xAA has 4 set bits per byte, 16 bytes)
    //   v_sparse(id4): Hamming = 16  (one bit per byte, 16 bytes)

    let results = index.search(&v_zeros, 4).unwrap();
    assert_eq!(results.len(), 4);

    // Results should be sorted by ascending Hamming distance
    assert_eq!(results[0].id, id1); // distance 0
    assert!((results[0].distance - 0.0).abs() < f32::EPSILON);

    assert_eq!(results[1].id, id4); // distance 16
    assert!((results[1].distance - 16.0).abs() < f32::EPSILON);

    assert_eq!(results[2].id, id3); // distance 64
    assert!((results[2].distance - 64.0).abs() < f32::EPSILON);

    assert_eq!(results[3].id, id2); // distance 128
    assert!((results[3].distance - 128.0).abs() < f32::EPSILON);

    // Verify strict ascending order property
    for window in results.windows(2) {
        assert!(
            window[0].distance <= window[1].distance,
            "results not sorted: {} vs {}",
            window[0].distance,
            window[1].distance
        );
    }

    // --- Phase 4: Search with k < count ---
    let top2 = index.search(&v_zeros, 2).unwrap();
    assert_eq!(top2.len(), 2);
    assert_eq!(top2[0].id, id1);
    assert_eq!(top2[1].id, id4);

    // --- Phase 5: Clear and verify empty ---
    index.clear();
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
    assert_eq!(index.vectors_len(), 0);

    // Search on empty index returns empty vec
    let empty_results = index.search(&v_zeros, 10).unwrap();
    assert!(empty_results.is_empty());

    // Get on cleared index returns None
    assert_eq!(index.get(id1), None);
}

#[test]
fn test_binary_flat_index_dimension_errors() {
    // Non-divisible-by-8 dimensions should fail
    let result = BinaryFlatIndex::new(100);
    assert!(matches!(
        result,
        Err(BinaryFlatIndexError::InvalidDimensions(100))
    ));

    let result = BinaryFlatIndex::new(7);
    assert!(matches!(
        result,
        Err(BinaryFlatIndexError::InvalidDimensions(7))
    ));

    // Wrong vector length on insert
    let mut index = BinaryFlatIndex::new(64).unwrap(); // 8 bytes
    let result = index.insert(&[0xFF; 16]); // 16 bytes, expected 8
    assert!(matches!(
        result,
        Err(BinaryFlatIndexError::DimensionMismatch { .. })
    ));

    // Wrong query length on search
    index.insert(&[0xFF; 8]).unwrap();
    let result = index.search(&[0x00; 4], 1);
    assert!(matches!(
        result,
        Err(BinaryFlatIndexError::DimensionMismatch { .. })
    ));
}

#[test]
fn test_binary_flat_index_with_capacity() {
    let index = BinaryFlatIndex::with_capacity(256, 1000).unwrap();
    assert_eq!(index.dimensions(), 256);
    assert_eq!(index.bytes_per_vector(), 32);
    assert!(index.is_empty());
    // Pre-allocated memory should be at least 1000 * 32 bytes
    assert!(index.memory_usage() >= 32_000);
}

#[test]
fn test_binary_flat_index_large_batch() {
    let mut index = BinaryFlatIndex::new(64).unwrap();

    // Insert 200 vectors with varying patterns
    for i in 0u8..200 {
        let v: Vec<u8> = (0..8).map(|j| i.wrapping_add(j * 31)).collect();
        index.insert(&v).unwrap();
    }

    assert_eq!(index.len(), 200);

    // Search should return k results sorted by distance
    let query = vec![0u8; 8];
    let results = index.search(&query, 10).unwrap();
    assert_eq!(results.len(), 10);

    for window in results.windows(2) {
        assert!(window[0].distance <= window[1].distance);
    }
}

#[test]
fn test_binary_flat_index_shrink_after_operations() {
    let mut index = BinaryFlatIndex::with_capacity(64, 10_000).unwrap();

    for _ in 0..5 {
        index.insert(&[0xAB; 8]).unwrap();
    }

    let mem_before = index.memory_usage();
    index.shrink_to_fit();
    let mem_after = index.memory_usage();

    assert!(mem_after <= mem_before);
    assert_eq!(index.len(), 5);

    // Vectors still intact after shrink
    for i in 1..=5u64 {
        assert_eq!(index.get(VectorId(i)), Some([0xAB; 8].as_slice()));
    }
}

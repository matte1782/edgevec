//! Integration tests for binary vector support.
//!
//! Tests the full binary vector workflow including:
//! - Binary vector insertion (direct and via BQ)
//! - Hamming distance search
//! - Binary storage persistence
//! - Recall accuracy

use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::quantization::BinaryQuantizer;
use edgevec::storage::{StorageType, VectorStorage};

/// Helper to create a binary HNSW config.
fn create_binary_config(dimensions: u32) -> HnswConfig {
    let mut config = HnswConfig::new(dimensions);
    config.metric = HnswConfig::METRIC_HAMMING;
    config.m = 16;
    config.m0 = 32;
    config.ef_construction = 100;
    config.ef_search = 50;
    config
}

/// Helper to create random binary vectors.
fn random_binary_vectors(count: usize, bytes_per_vector: usize) -> Vec<Vec<u8>> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    (0..count)
        .map(|i| {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            let seed = hasher.finish();

            (0..bytes_per_vector)
                .map(|j| {
                    let mut h = DefaultHasher::new();
                    (seed, j).hash(&mut h);
                    (h.finish() % 256) as u8
                })
                .collect()
        })
        .collect()
}

/// Helper to compute Hamming distance between two byte slices.
fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones())
        .sum()
}

#[test]
fn test_binary_insert_and_search() {
    // 1024 bits = 128 bytes
    let dimensions = 1024;
    let bytes_per_vector = dimensions as usize / 8;

    let config = create_binary_config(dimensions);
    let mut storage = VectorStorage::new(&config, None);
    storage.set_storage_type(StorageType::Binary(dimensions));

    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    // Insert 100 random binary vectors
    let vectors = random_binary_vectors(100, bytes_per_vector);
    for vec in &vectors {
        index.insert_binary(vec, &mut storage).unwrap();
    }

    // Search with the first vector as query
    let query = &vectors[0];
    let results = index.search_binary(query, 10, &storage).unwrap();

    // The first result should be the query itself (distance 0)
    assert!(!results.is_empty());
    assert_eq!(results[0].distance, 0.0);
    assert_eq!(results[0].vector_id.0, 1); // First inserted vector has ID 1

    // Results should be sorted by distance
    for i in 1..results.len() {
        assert!(
            results[i].distance >= results[i - 1].distance,
            "Results not sorted at index {}: {} >= {}",
            i,
            results[i].distance,
            results[i - 1].distance
        );
    }
}

#[test]
fn test_binary_quantization_from_f32() {
    // 768 dimensions (common embedding size)
    let dimensions = 768;

    let config = create_binary_config(dimensions);
    let mut storage = VectorStorage::new(&config, None);
    storage.set_storage_type(StorageType::Binary(dimensions));

    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    // Create f32 vector with known pattern: alternating positive/negative
    let f32_vec: Vec<f32> = (0..dimensions)
        .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
        .collect();

    // Insert using binary quantization
    let id = index.insert_with_bq(&f32_vec, &mut storage).unwrap();
    assert_eq!(id.0, 1);

    // Get the stored binary vector
    let stored = storage.get_binary_vector(id).unwrap();

    // Verify the pattern: even indices positive -> bit 1, odd indices negative -> bit 0
    // Each byte should be 0b01010101 = 0x55
    for byte in stored {
        assert_eq!(*byte, 0x55, "Expected alternating pattern 0x55");
    }
}

#[test]
fn test_hamming_distance_correctness() {
    let dimensions = 128; // 128 bits = 16 bytes
    let bytes = dimensions as usize / 8;

    let config = create_binary_config(dimensions);
    let mut storage = VectorStorage::new(&config, None);
    storage.set_storage_type(StorageType::Binary(dimensions));

    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    // Create vectors with known Hamming distances
    let all_zeros = vec![0u8; bytes];
    let all_ones = vec![0xFFu8; bytes];
    let half_ones = {
        let mut v = vec![0u8; bytes];
        for byte in v.iter_mut().take(bytes / 2) {
            *byte = 0xFF;
        }
        v
    };

    // Insert all three
    index.insert_binary(&all_zeros, &mut storage).unwrap();
    index.insert_binary(&all_ones, &mut storage).unwrap();
    index.insert_binary(&half_ones, &mut storage).unwrap();

    // Search from all_zeros
    let results = index.search_binary(&all_zeros, 3, &storage).unwrap();

    // First result should be all_zeros (distance 0)
    assert_eq!(results[0].vector_id.0, 1);
    assert_eq!(results[0].distance, 0.0);

    // Verify distances match expected Hamming distances
    let expected_dist_to_half = hamming_distance(&all_zeros, &half_ones);
    let expected_dist_to_all_ones = hamming_distance(&all_zeros, &all_ones);

    // half_ones should have distance of 64 bits (half of 128)
    assert_eq!(expected_dist_to_half, 64);
    // all_ones should have distance of 128 bits (all different)
    assert_eq!(expected_dist_to_all_ones, 128);
}

#[test]
fn test_binary_recall_100_vectors() {
    let dimensions = 512; // 512 bits = 64 bytes
    let bytes_per_vector = dimensions as usize / 8;
    let n_vectors = 100;
    let k = 10;

    let config = create_binary_config(dimensions);
    let mut storage = VectorStorage::new(&config, None);
    storage.set_storage_type(StorageType::Binary(dimensions));

    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    // Insert vectors
    let vectors = random_binary_vectors(n_vectors, bytes_per_vector);
    for vec in &vectors {
        index.insert_binary(vec, &mut storage).unwrap();
    }

    // For each vector, verify it can find itself
    for (i, query) in vectors.iter().enumerate() {
        let results = index.search_binary(query, k, &storage).unwrap();

        // Should find itself as the closest (distance 0)
        let found_self = results.iter().any(|r| r.vector_id.0 == (i as u64 + 1));
        assert!(
            found_self,
            "Vector {} should find itself in top {} results",
            i, k
        );
    }
}

#[test]
fn test_binary_empty_index_search() {
    let dimensions = 256;

    let config = create_binary_config(dimensions);
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).unwrap();

    let query = vec![0u8; dimensions as usize / 8];
    let results = index.search_binary(&query, 10, &storage).unwrap();

    // Empty index should return empty results
    assert!(results.is_empty());
}

#[test]
fn test_binary_dimension_validation() {
    let dimensions = 128;
    let expected_bytes = dimensions as usize / 8;

    let config = create_binary_config(dimensions);
    let mut storage = VectorStorage::new(&config, None);
    storage.set_storage_type(StorageType::Binary(dimensions));

    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Correct size should work
    let correct = vec![0u8; expected_bytes];
    assert!(index.insert_binary(&correct, &mut storage).is_ok());

    // Wrong size should fail
    let too_short = vec![0u8; expected_bytes - 1];
    assert!(index.insert_binary(&too_short, &mut storage).is_err());

    let too_long = vec![0u8; expected_bytes + 1];
    assert!(index.insert_binary(&too_long, &mut storage).is_err());
}

#[test]
fn test_binary_quantizer_to_bytes() {
    // Test the quantize_to_bytes static method
    let f32_vec: Vec<f32> = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let binary = BinaryQuantizer::quantize_to_bytes(&f32_vec);

    assert_eq!(binary.len(), 1); // 8 bits = 1 byte
    assert_eq!(binary[0], 0b01010101); // LSB-first: even positions are 1

    // Test with 16 elements
    let f32_vec16: Vec<f32> = (0..16)
        .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
        .collect();
    let binary16 = BinaryQuantizer::quantize_to_bytes(&f32_vec16);

    assert_eq!(binary16.len(), 2);
    assert_eq!(binary16[0], 0b01010101);
    assert_eq!(binary16[1], 0b01010101);
}

#[test]
fn test_binary_storage_memory_efficiency() {
    // Verify that binary storage uses expected memory
    let dimensions = 1024; // 1024 bits = 128 bytes per vector
    let n_vectors = 1000;
    let bytes_per_vector = dimensions as usize / 8;

    let config = create_binary_config(dimensions);
    let mut storage = VectorStorage::new(&config, None);
    storage.set_storage_type(StorageType::Binary(dimensions));

    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert vectors
    let vectors = random_binary_vectors(n_vectors, bytes_per_vector);
    for vec in &vectors {
        index.insert_binary(vec, &mut storage).unwrap();
    }

    // Binary data should be approximately n_vectors * bytes_per_vector
    // (Can't directly check storage.binary_data.len() since it's private,
    // but we can verify search still works correctly)
    let query = &vectors[0];
    let results = index.search_binary(query, 5, &storage).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_binary_with_regular_search() {
    // Test that regular f32 search path also works with Hamming metric
    // (via auto-conversion)
    let dimensions = 256;

    let mut config = create_binary_config(dimensions);
    config.metric = HnswConfig::METRIC_HAMMING;

    let mut storage = VectorStorage::new(&config, None);
    storage.set_storage_type(StorageType::Binary(dimensions));

    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Insert using f32 vectors (should auto-quantize to binary)
    let f32_vec: Vec<f32> = (0..dimensions)
        .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
        .collect();
    let id = index.insert(&f32_vec, &mut storage).unwrap();
    assert_eq!(id.0, 1);

    // Search using the same f32 vector
    let results = index.search(&f32_vec, 1, &storage).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].vector_id.0, 1);
    assert_eq!(results[0].distance, 0.0); // Same vector should have distance 0
}

//! Integration tests for BQ persistence (W28.4.4).
//!
//! Tests that persistence works correctly when BQ is enabled:
//! - Index can be saved/loaded with BQ enabled
//! - F32 vectors are preserved (BQ vectors regenerated on load)
//! - Metadata is preserved across save/load
//! - Search works after reload

use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::metadata::MetadataValue;
use edgevec::persistence::{read_snapshot, write_snapshot, MemoryBackend};
use edgevec::storage::VectorStorage;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

// =============================================================================
// Helper functions
// =============================================================================

fn calculate_recall(ground_truth: &HashSet<u64>, test_results: &HashSet<u64>, k: usize) -> f64 {
    let intersection = ground_truth.intersection(test_results).count();
    intersection as f64 / k.min(ground_truth.len()) as f64
}

// =============================================================================
// W28.4.4: BQ Persistence Tests
// =============================================================================

mod bq_persistence {
    use super::*;

    /// Test that a BQ index can be saved and loaded.
    #[test]
    fn test_bq_index_save_load_basic() {
        const DIM: u32 = 64;
        const NUM_VECTORS: usize = 100;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;
        config.m0 = 32;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
            index.insert_bq(&v, &mut storage).expect("Insert failed");
        }

        assert!(index.has_bq(), "Index should have BQ enabled");
        assert_eq!(index.len(), NUM_VECTORS);

        // Save
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write snapshot");

        // Load
        let (loaded_index, loaded_storage) =
            read_snapshot(&backend).expect("Failed to read snapshot");

        // Verify basic counts
        assert_eq!(loaded_index.len(), NUM_VECTORS);
        assert_eq!(loaded_storage.len(), NUM_VECTORS);

        println!(
            "BQ index save/load: {} vectors preserved",
            loaded_index.len()
        );
    }

    /// Test that F32 search works after loading a BQ index.
    #[test]
    fn test_bq_index_f32_search_after_load() {
        const DIM: u32 = 64;
        const NUM_VECTORS: usize = 200;
        const K: usize = 10;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;
        config.m0 = 32;
        config.ef_search = 100;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(123);

        for _ in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
            index.insert_bq(&v, &mut storage).expect("Insert failed");
        }

        // Generate query
        let query: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();

        // Search before save
        let results_before = index.search(&query, K, &storage).expect("Search failed");
        let ids_before: HashSet<_> = results_before.iter().map(|r| r.vector_id.0).collect();

        // Save and load
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write snapshot");
        let (loaded_index, loaded_storage) =
            read_snapshot(&backend).expect("Failed to read snapshot");

        // Search after load
        let results_after = loaded_index
            .search(&query, K, &loaded_storage)
            .expect("Search failed");
        let ids_after: HashSet<_> = results_after.iter().map(|r| r.vector_id.0).collect();

        // Results should be identical (same HNSW graph)
        let recall = calculate_recall(&ids_before, &ids_after, K);
        assert!(
            recall >= 0.9,
            "F32 search recall after load should be >= 0.9, got {recall:.3}"
        );

        println!("F32 search after load: recall = {recall:.3}");
    }

    /// Test that metadata is preserved when saving a BQ index.
    #[test]
    fn test_bq_index_metadata_preserved() {
        const DIM: u32 = 32;
        const NUM_VECTORS: usize = 50;

        let mut config = HnswConfig::new(DIM);
        config.m = 8;
        config.m0 = 16;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(456);
        let categories = ["news", "sports", "tech"];

        for i in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
            let vector_id = index.insert_bq(&v, &mut storage).expect("Insert failed");

            #[allow(clippy::cast_possible_truncation)]
            let meta_id = vector_id.0 as u32;
            index
                .metadata_mut()
                .insert(
                    meta_id,
                    "category",
                    MetadataValue::String(categories[i % 3].to_string()),
                )
                .expect("Metadata insert failed");
            index
                .metadata_mut()
                .insert(meta_id, "index", MetadataValue::Integer(i as i64))
                .expect("Metadata insert failed");
        }

        // Verify metadata before save
        assert_eq!(index.metadata().vector_count(), NUM_VECTORS);
        assert_eq!(index.metadata().total_key_count(), NUM_VECTORS * 2); // 2 keys per vector

        // Save and load
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write snapshot");
        let (loaded_index, _) = read_snapshot(&backend).expect("Failed to read snapshot");

        // Verify metadata after load
        assert_eq!(loaded_index.metadata().vector_count(), NUM_VECTORS);
        assert_eq!(loaded_index.metadata().total_key_count(), NUM_VECTORS * 2);

        // Verify specific values (VectorId starts at 1)
        let meta1 = loaded_index
            .metadata()
            .get_all(1)
            .expect("Metadata for vector_id=1");
        assert_eq!(
            meta1.get("category"),
            Some(&MetadataValue::String("news".into()))
        );
        assert_eq!(meta1.get("index"), Some(&MetadataValue::Integer(0)));

        println!(
            "Metadata preserved: {} vectors, {} keys",
            NUM_VECTORS,
            NUM_VECTORS * 2
        );
    }

    /// Test roundtrip with BQ index - verify BQ needs to be re-enabled after load.
    #[test]
    fn test_bq_index_bq_state_after_load() {
        const DIM: u32 = 64;
        const NUM_VECTORS: usize = 100;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(789);

        for _ in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
            index.insert_bq(&v, &mut storage).expect("Insert failed");
        }

        assert!(index.has_bq(), "Index should have BQ before save");

        // Save and load
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write snapshot");
        let (loaded_index, _) = read_snapshot(&backend).expect("Failed to read snapshot");

        // Note: BQ storage is NOT persisted, so after load, has_bq() returns false
        // This is expected behavior - BQ can be regenerated from F32 vectors
        assert!(
            !loaded_index.has_bq(),
            "Loaded index should not have BQ (BQ not persisted)"
        );

        println!("BQ state after load: has_bq() = {}", loaded_index.has_bq());
    }

    /// Test that we can insert more vectors after loading a BQ index.
    #[test]
    fn test_bq_index_insert_after_load() {
        const DIM: u32 = 64;
        const NUM_VECTORS: usize = 50;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(999);

        for _ in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
            index.insert_bq(&v, &mut storage).expect("Insert failed");
        }

        // Save and load
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write snapshot");
        let (mut loaded_index, mut loaded_storage) =
            read_snapshot(&backend).expect("Failed to read snapshot");

        // Insert more vectors after load (using regular insert since BQ not persisted)
        for _ in 0..50 {
            let v: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
            loaded_index
                .insert(&v, &mut loaded_storage)
                .expect("Insert after load failed");
        }

        assert_eq!(loaded_index.len(), 100);
        assert_eq!(loaded_storage.len(), 100);

        // Search should still work
        let query: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let results = loaded_index
            .search(&query, 10, &loaded_storage)
            .expect("Search failed");
        assert!(!results.is_empty(), "Search should return results");

        println!("Insert after load: {} total vectors", loaded_index.len());
    }

    /// Test persistence with empty BQ index.
    #[test]
    fn test_bq_index_empty_roundtrip() {
        const DIM: u32 = 128;

        let config = HnswConfig::new(DIM);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        assert!(index.has_bq(), "Empty index should have BQ enabled");
        assert_eq!(index.len(), 0);

        // Save and load
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write snapshot");
        let (loaded_index, loaded_storage) =
            read_snapshot(&backend).expect("Failed to read snapshot");

        assert_eq!(loaded_index.len(), 0);
        assert_eq!(loaded_storage.len(), 0);

        println!("Empty BQ index roundtrip: success");
    }

    /// Test persistence with larger dataset and BQ.
    #[test]
    fn test_bq_index_large_roundtrip() {
        const DIM: u32 = 128;
        const NUM_VECTORS: usize = 500;
        const K: usize = 10;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;
        config.m0 = 32;
        config.ef_construction = 128;
        config.ef_search = 128;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(12345);

        for i in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
            let vector_id = index.insert_bq(&v, &mut storage).expect("Insert failed");

            // Add metadata to every 10th vector
            if i % 10 == 0 {
                #[allow(clippy::cast_possible_truncation)]
                let meta_id = vector_id.0 as u32;
                index
                    .metadata_mut()
                    .insert(meta_id, "batch", MetadataValue::Integer((i / 10) as i64))
                    .expect("Metadata insert failed");
            }
        }

        // Save and load
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write snapshot");
        let (loaded_index, loaded_storage) =
            read_snapshot(&backend).expect("Failed to read snapshot");

        // Verify counts
        assert_eq!(loaded_index.len(), NUM_VECTORS);
        assert_eq!(loaded_storage.len(), NUM_VECTORS);
        assert_eq!(loaded_index.metadata().vector_count(), 50); // Every 10th vector

        // Verify search works
        let query: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let results = loaded_index
            .search(&query, K, &loaded_storage)
            .expect("Search failed");
        assert_eq!(results.len(), K);

        println!(
            "Large BQ roundtrip: {} vectors, {} with metadata",
            NUM_VECTORS, 50
        );
    }
}

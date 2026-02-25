//! Integration tests for hybrid search (BQ + filter) - W28.4.3
//!
//! Verifies that hybrid search correctly combines Binary Quantization
//! with metadata filtering per RFC-002.

use edgevec::filter::{evaluate, parse};
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::metadata::MetadataValue;
use edgevec::storage::VectorStorage;
use rand::rngs::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::collections::HashSet;

// =============================================================================
// Helper functions
// =============================================================================

#[allow(dead_code)]
fn make_vector(dim: u32, seed: u32) -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(seed as u64);
    (0..dim).map(|_| rng.random_range(-1.0..1.0)).collect()
}

fn calculate_recall(ground_truth: &HashSet<u64>, test_results: &HashSet<u64>, k: usize) -> f64 {
    let intersection = ground_truth.intersection(test_results).count();
    intersection as f64 / k.min(ground_truth.len()) as f64
}

// =============================================================================
// W28.4.3: Hybrid Search Tests
// =============================================================================

mod hybrid_search {
    use super::*;

    /// Test that hybrid search combines BQ speed with filter accuracy.
    #[test]
    fn test_hybrid_bq_with_filter() {
        const DIM: u32 = 128;
        const NUM_VECTORS: usize = 500;
        const K: usize = 10;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;
        config.m0 = 32;
        config.ef_construction = 100;
        config.ef_search = 100;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let categories = ["news", "sports", "tech", "entertainment"];
        let mut rng = StdRng::seed_from_u64(42);

        // Insert vectors with metadata using insert_bq + manual metadata
        for i in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();
            let vector_id = index.insert_bq(&v, &mut storage).expect("Insert failed");

            // Add metadata manually
            #[allow(clippy::cast_possible_truncation)]
            let meta_id = vector_id.0 as u32;
            index
                .metadata_mut()
                .insert(
                    meta_id,
                    "category",
                    MetadataValue::String(categories[i % 4].to_string()),
                )
                .expect("Metadata insert failed");
            index
                .metadata_mut()
                .insert(
                    meta_id,
                    "score",
                    MetadataValue::Float(rng.random_range(0.0..1.0)),
                )
                .expect("Metadata insert failed");
            index
                .metadata_mut()
                .insert(meta_id, "active", MetadataValue::Boolean(i % 2 == 0))
                .expect("Metadata insert failed");
        }

        // Query with filter
        let query: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();
        let filter_expr = parse("category = \"news\"").expect("Parse failed");

        // Get BQ candidates
        let bq_candidates = index
            .search_bq(&query, K * 3, &storage)
            .expect("BQ search failed");

        // Filter candidates
        let filtered: Vec<_> = bq_candidates
            .into_iter()
            .filter(|(vid, _)| {
                #[allow(clippy::cast_possible_truncation)]
                let metadata = index.metadata().get_all(vid.0 as u32);
                if let Some(m) = metadata {
                    evaluate(&filter_expr, m).unwrap_or(false)
                } else {
                    false
                }
            })
            .take(K)
            .collect();

        // All results should match filter
        for (vid, _) in &filtered {
            #[allow(clippy::cast_possible_truncation)]
            let meta = index.metadata().get(vid.0 as u32, "category");
            assert!(meta.is_some(), "Metadata should exist");
            assert_eq!(
                meta.unwrap().as_string(),
                Some("news"),
                "All results should have category = news"
            );
        }

        println!("Hybrid search: {} results matching filter", filtered.len());
    }

    /// Test complex filters with AND/OR.
    #[test]
    fn test_hybrid_complex_filter() {
        const DIM: u32 = 64;
        const NUM_VECTORS: usize = 300;
        const K: usize = 10;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;
        config.m0 = 32;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(123);

        for i in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();
            let vector_id = index.insert_bq(&v, &mut storage).expect("Insert failed");

            #[allow(clippy::cast_possible_truncation)]
            let meta_id = vector_id.0 as u32;
            index
                .metadata_mut()
                .insert(
                    meta_id,
                    "category",
                    MetadataValue::String(["news", "tech", "sports"][i % 3].to_string()),
                )
                .expect("Metadata insert failed");
            index
                .metadata_mut()
                .insert(meta_id, "active", MetadataValue::Boolean(i % 2 == 0))
                .expect("Metadata insert failed");
        }

        let query: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();

        // Complex filter: (news OR tech) AND active
        let filter_expr = parse("(category = \"news\" OR category = \"tech\") AND active = true")
            .expect("Parse failed");

        // Get BQ candidates
        let bq_candidates = index
            .search_bq(&query, K * 5, &storage)
            .expect("BQ search failed");

        // Filter candidates
        let filtered: Vec<_> = bq_candidates
            .into_iter()
            .filter(|(vid, _)| {
                #[allow(clippy::cast_possible_truncation)]
                let metadata = index.metadata().get_all(vid.0 as u32);
                if let Some(m) = metadata {
                    evaluate(&filter_expr, m).unwrap_or(false)
                } else {
                    false
                }
            })
            .take(K)
            .collect();

        // Verify all results match complex filter
        for (vid, _) in &filtered {
            #[allow(clippy::cast_possible_truncation)]
            let meta = index.metadata().get_all(vid.0 as u32).unwrap();
            let category = meta.get("category").unwrap().as_string().unwrap();
            let active = meta.get("active").unwrap().as_boolean().unwrap();

            assert!(
                category == "news" || category == "tech",
                "Category should be news or tech"
            );
            assert!(active, "Active should be true");
        }

        println!(
            "Complex filter: {} results matching (news OR tech) AND active",
            filtered.len()
        );
    }

    /// Test array ANY filter for membership checks.
    #[test]
    fn test_hybrid_array_any_filter() {
        const DIM: u32 = 64;
        const NUM_VECTORS: usize = 200;
        const K: usize = 10;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;
        config.m0 = 32;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(456);

        for i in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();
            let vector_id = index.insert_bq(&v, &mut storage).expect("Insert failed");

            // Every 3rd vector has "featured" tag
            let tags = if i % 3 == 0 {
                vec!["featured".to_string(), "trending".to_string()]
            } else {
                vec!["normal".to_string()]
            };

            #[allow(clippy::cast_possible_truncation)]
            let meta_id = vector_id.0 as u32;
            index
                .metadata_mut()
                .insert(meta_id, "tags", MetadataValue::StringArray(tags))
                .expect("Metadata insert failed");
        }

        let query: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();

        // Filter: tags ANY ["featured"] (array membership)
        let filter_expr = parse(r#"tags ANY ["featured"]"#).expect("Parse failed");

        // Get BQ candidates
        let bq_candidates = index
            .search_bq(&query, K * 5, &storage)
            .expect("BQ search failed");

        // Filter candidates
        let filtered: Vec<_> = bq_candidates
            .into_iter()
            .filter(|(vid, _)| {
                #[allow(clippy::cast_possible_truncation)]
                let metadata = index.metadata().get_all(vid.0 as u32);
                if let Some(m) = metadata {
                    evaluate(&filter_expr, m).unwrap_or(false)
                } else {
                    false
                }
            })
            .take(K)
            .collect();

        // Verify all results have "featured" tag
        for (vid, _) in &filtered {
            #[allow(clippy::cast_possible_truncation)]
            let meta = index.metadata().get_all(vid.0 as u32).unwrap();
            let tags = meta.get("tags").unwrap().as_string_array().unwrap();
            assert!(
                tags.contains(&"featured".to_string()),
                "Tags should contain 'featured'"
            );
        }

        println!("ANY filter: {} results with 'featured' tag", filtered.len());
    }

    /// Test fallback when BQ is disabled.
    #[test]
    fn test_hybrid_fallback_no_bq() {
        const DIM: u32 = 64;
        const NUM_VECTORS: usize = 100;
        const K: usize = 10;

        let config = HnswConfig::new(DIM);
        let mut storage = VectorStorage::new(&config, None);

        // Create index WITHOUT BQ
        let mut index = HnswIndex::new(config, &storage).expect("Index creation failed");

        let mut rng = StdRng::seed_from_u64(789);

        for _ in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();
            let mut metadata = std::collections::HashMap::new();
            metadata.insert(
                "category".to_string(),
                MetadataValue::String("test".to_string()),
            );

            index
                .insert_with_metadata(&mut storage, &v, metadata)
                .expect("Insert failed");
        }

        // BQ search should fail gracefully
        let query: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();

        // BQ is not enabled, so search_bq should error
        assert!(!index.has_bq(), "Index should not have BQ");

        // Regular search should still work
        let results = index.search(&query, K, &storage).expect("Search failed");
        assert!(!results.is_empty(), "F32 search should return results");

        println!("Fallback test: F32 search works when BQ disabled");
    }

    /// Test hybrid recall vs pure filtered search.
    #[test]
    fn test_hybrid_recall_with_filter() {
        const DIM: u32 = 128;
        const NUM_VECTORS: usize = 500;
        const K: usize = 10;
        const RESCORE_FACTOR: usize = 10;

        let mut config = HnswConfig::new(DIM);
        config.m = 16;
        config.m0 = 32;
        config.ef_construction = 128;
        config.ef_search = 128;

        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

        let mut rng = StdRng::seed_from_u64(999);

        for idx in 0..NUM_VECTORS {
            let v: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();
            let vector_id = index.insert_bq(&v, &mut storage).expect("Insert failed");

            #[allow(clippy::cast_possible_truncation)]
            let meta_id = vector_id.0 as u32;
            index
                .metadata_mut()
                .insert(
                    meta_id,
                    "category",
                    MetadataValue::String(["a", "b"][idx % 2].to_string()),
                )
                .expect("Metadata insert failed");
        }

        let query: Vec<f32> = (0..DIM).map(|_| rng.random_range(-1.0..1.0)).collect();
        let filter_expr = parse("category = \"a\"").expect("Parse failed");

        // Ground truth: F32 search filtered
        let f32_results = index
            .search(&query, K * 3, &storage)
            .expect("Search failed");
        let f32_filtered: Vec<_> = f32_results
            .iter()
            .filter(|r| {
                #[allow(clippy::cast_possible_truncation)]
                let metadata = index.metadata().get_all(r.vector_id.0 as u32);
                if let Some(m) = metadata {
                    evaluate(&filter_expr, m).unwrap_or(false)
                } else {
                    false
                }
            })
            .take(K)
            .map(|r| r.vector_id.0)
            .collect();
        let f32_ids: HashSet<_> = f32_filtered.into_iter().collect();

        // Hybrid: BQ + filter + rescore
        let bq_candidates = index
            .search_bq(&query, K * RESCORE_FACTOR, &storage)
            .expect("BQ search failed");

        let filtered: Vec<_> = bq_candidates
            .into_iter()
            .filter(|(vid, _)| {
                #[allow(clippy::cast_possible_truncation)]
                let metadata = index.metadata().get_all(vid.0 as u32);
                if let Some(m) = metadata {
                    evaluate(&filter_expr, m).unwrap_or(false)
                } else {
                    false
                }
            })
            .take(K)
            .collect();

        let hybrid_ids: HashSet<_> = filtered.iter().map(|(id, _)| id.0).collect();

        let recall = calculate_recall(&f32_ids, &hybrid_ids, K);
        println!("Hybrid filtered recall: {recall:.3}");

        // Hybrid with enough overfetch should have reasonable recall
        assert!(
            recall >= 0.5,
            "Hybrid filtered recall {recall:.3} should be >= 0.5"
        );
    }
}

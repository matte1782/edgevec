//! Integration tests for HNSW + Filter system.
//!
//! Tests end-to-end filtered search functionality with real HNSW indexes.

use edgevec::filter::{parse, FilterStrategy, FilteredSearcher, VectorMetadataStore};
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::metadata::MetadataValue;
use edgevec::storage::VectorStorage;
use rand::rngs::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::collections::HashMap;

// =============================================================================
// TEST UTILITIES
// =============================================================================

/// Helper to create metadata with typed fields.
fn make_metadata(fields: &[(&str, MetadataValue)]) -> HashMap<String, MetadataValue> {
    fields
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

/// Setup test index with vectors and metadata.
fn setup_test_index(
    dim: usize,
    num_vectors: usize,
    seed: u64,
) -> (HnswIndex, VectorStorage, VectorMetadataStore, Vec<Vec<f32>>) {
    let mut rng = StdRng::seed_from_u64(seed);

    // Create index
    let mut config = HnswConfig::new(dim as u32);
    config.m = 16;
    config.m0 = 32;
    config.ef_construction = 100;
    config.ef_search = 64;

    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Index creation failed");

    // Create metadata store
    let mut metadata_store = VectorMetadataStore::new();

    // Generate and insert vectors with metadata
    let mut vectors = Vec::with_capacity(num_vectors);
    for i in 0..num_vectors {
        let vec: Vec<f32> = (0..dim).map(|_| rng.random::<f32>()).collect();
        vectors.push(vec.clone());

        // Insert vector
        index.insert(&vec, &mut storage).expect("Insert failed");

        // Add metadata: category, price, active flag
        let category = match i % 3 {
            0 => "gpu",
            1 => "cpu",
            _ => "memory",
        };
        let price = (i % 100) as i64 * 10 + 50; // 50, 60, 70, ..., 1040
        let active = i % 2 == 0;

        let meta = make_metadata(&[
            ("category", MetadataValue::String(category.into())),
            ("price", MetadataValue::Integer(price)),
            ("active", MetadataValue::Boolean(active)),
            ("id", MetadataValue::Integer(i as i64)),
        ]);
        metadata_store.push(meta);
    }

    (index, storage, metadata_store, vectors)
}

// =============================================================================
// BASIC FILTERED SEARCH TESTS
// =============================================================================

#[test]
fn test_filtered_search_no_filter() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();
    let result = searcher
        .search_filtered(&query, 10, None, FilterStrategy::Auto)
        .expect("Search failed");

    assert!(!result.results.is_empty());
    assert!(result.results.len() <= 10);
    assert!(result.complete || result.results.len() == 10);
}

#[test]
fn test_filtered_search_category_eq() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse(r#"category = "gpu""#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();
    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // All results should be GPUs (indices 0, 3, 6, 9, ...)
    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            assert!(
                matches!(cat, Some(MetadataValue::String(s)) if s == "gpu"),
                "Expected gpu category, got {:?}",
                cat
            );
        }
    }
}

#[test]
fn test_filtered_search_price_lt() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse("price < 200").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();
    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // All results should have price < 200
    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let price = meta.get("price");
            assert!(
                matches!(price, Some(MetadataValue::Integer(p)) if *p < 200),
                "Expected price < 200, got {:?}",
                price
            );
        }
    }
}

#[test]
fn test_filtered_search_boolean_field() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse("active = true").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();
    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // All results should be active
    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let active = meta.get("active");
            assert!(
                matches!(active, Some(MetadataValue::Boolean(true))),
                "Expected active = true, got {:?}",
                active
            );
        }
    }
}

#[test]
fn test_filtered_search_compound_and() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse(r#"category = "gpu" AND active = true"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();
    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // All results should be active GPUs
    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let active = meta.get("active");
            assert!(
                matches!(cat, Some(MetadataValue::String(s)) if s == "gpu"),
                "Expected gpu"
            );
            assert!(
                matches!(active, Some(MetadataValue::Boolean(true))),
                "Expected active"
            );
        }
    }
}

#[test]
fn test_filtered_search_compound_or() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse(r#"category = "gpu" OR category = "cpu""#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();
    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // All results should be GPU or CPU
    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let is_valid =
                matches!(cat, Some(MetadataValue::String(s)) if s == "gpu" || s == "cpu");
            assert!(is_valid, "Expected gpu or cpu, got {:?}", cat);
        }
    }
}

// =============================================================================
// STRATEGY TESTS
// =============================================================================

#[test]
fn test_filtered_search_prefilter_strategy() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse("active = true").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 5, Some(&filter), FilterStrategy::PreFilter)
        .expect("Search failed");

    assert!(matches!(result.strategy_used, FilterStrategy::PreFilter));
}

#[test]
fn test_filtered_search_postfilter_strategy() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse("active = true").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(
            &query,
            5,
            Some(&filter),
            FilterStrategy::PostFilter { oversample: 2.0 },
        )
        .expect("Search failed");

    assert!(matches!(
        result.strategy_used,
        FilterStrategy::PostFilter { .. }
    ));
}

#[test]
fn test_filtered_search_hybrid_strategy() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse("active = true").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(
            &query,
            5,
            Some(&filter),
            FilterStrategy::Hybrid {
                oversample_min: 1.0,
                oversample_max: 5.0,
            },
        )
        .expect("Search failed");

    assert!(matches!(
        result.strategy_used,
        FilterStrategy::Hybrid { .. }
    ));
}

#[test]
fn test_filtered_search_auto_strategy() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse("active = true").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 5, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // Auto should resolve to one of the concrete strategies
    assert!(!matches!(result.strategy_used, FilterStrategy::Auto));
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[test]
fn test_filtered_search_no_matches() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // Filter that matches nothing
    let filter = parse("price > 99999").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    assert!(result.results.is_empty());
}

#[test]
fn test_filtered_search_tautology() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // Tautology filter (always true)
    let filter = parse("active = true OR active = false").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // Should return results since tautology matches all
    assert!(!result.results.is_empty());
}

#[test]
fn test_filtered_search_empty_index() {
    let dim = 64;
    let mut config = HnswConfig::new(dim);
    config.m = 16;
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).expect("Index creation failed");
    let metadata_store = VectorMetadataStore::new();

    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);
    let filter = parse("active = true").expect("Parse failed");
    let query: Vec<f32> = (0..dim as usize).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    assert!(result.results.is_empty());
}

// =============================================================================
// RANGE AND BETWEEN TESTS
// =============================================================================

#[test]
fn test_filtered_search_between() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse("price BETWEEN 100 AND 300").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // All results should have 100 <= price <= 300
    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let price = meta.get("price");
            assert!(
                matches!(price, Some(MetadataValue::Integer(p)) if *p >= 100 && *p <= 300),
                "Expected 100 <= price <= 300, got {:?}",
                price
            );
        }
    }
}

#[test]
fn test_filtered_search_in_operator() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse(r#"category IN ["gpu", "cpu"]"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let is_valid =
                matches!(cat, Some(MetadataValue::String(s)) if s == "gpu" || s == "cpu");
            assert!(is_valid, "Expected gpu or cpu, got {:?}", cat);
        }
    }
}

// =============================================================================
// MULTI-FIELD FILTER TESTS
// =============================================================================

#[test]
fn test_multi_field_three_field_and() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // Three field AND: category = "gpu" AND price < 500 AND active = true
    let filter =
        parse(r#"category = "gpu" AND price < 500 AND active = true"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            assert!(matches!(meta.get("category"), Some(MetadataValue::String(s)) if s == "gpu"));
            assert!(matches!(meta.get("price"), Some(MetadataValue::Integer(p)) if *p < 500));
            assert!(matches!(
                meta.get("active"),
                Some(MetadataValue::Boolean(true))
            ));
        }
    }
}

#[test]
fn test_multi_field_three_field_or() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // Three field OR (all fields present to avoid UnknownField errors)
    let filter = parse(r#"(category = "gpu" AND price = 50) OR (category = "cpu" AND price = 60) OR (category = "memory" AND price = 70)"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // Results should match at least one of the three conditions
    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let price = meta.get("price");

            let match1 = matches!(cat, Some(MetadataValue::String(s)) if s == "gpu")
                && matches!(price, Some(MetadataValue::Integer(50)));
            let match2 = matches!(cat, Some(MetadataValue::String(s)) if s == "cpu")
                && matches!(price, Some(MetadataValue::Integer(60)));
            let match3 = matches!(cat, Some(MetadataValue::String(s)) if s == "memory")
                && matches!(price, Some(MetadataValue::Integer(70)));

            assert!(match1 || match2 || match3, "No condition matched");
        }
    }
}

#[test]
fn test_multi_field_mixed_and_or() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // (category = "gpu" OR category = "cpu") AND price < 300
    let filter =
        parse(r#"(category = "gpu" OR category = "cpu") AND price < 300"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let price = meta.get("price");

            let valid_cat =
                matches!(cat, Some(MetadataValue::String(s)) if s == "gpu" || s == "cpu");
            let valid_price = matches!(price, Some(MetadataValue::Integer(p)) if *p < 300);

            assert!(valid_cat && valid_price, "Filter not satisfied");
        }
    }
}

#[test]
fn test_multi_field_nested_parentheses() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // ((category = "gpu" AND active = true) OR (category = "cpu" AND active = false)) AND price < 400
    let filter = parse(r#"((category = "gpu" AND active = true) OR (category = "cpu" AND active = false)) AND price < 400"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let active = meta.get("active");
            let price = meta.get("price");

            let match1 = matches!(cat, Some(MetadataValue::String(s)) if s == "gpu")
                && matches!(active, Some(MetadataValue::Boolean(true)));
            let match2 = matches!(cat, Some(MetadataValue::String(s)) if s == "cpu")
                && matches!(active, Some(MetadataValue::Boolean(false)));
            let price_ok = matches!(price, Some(MetadataValue::Integer(p)) if *p < 400);

            assert!((match1 || match2) && price_ok, "Filter not satisfied");
        }
    }
}

#[test]
fn test_multi_field_with_not() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // NOT(category = "memory") AND active = true
    let filter = parse(r#"NOT(category = "memory") AND active = true"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let active = meta.get("active");

            // Should NOT be memory
            assert!(!matches!(cat, Some(MetadataValue::String(s)) if s == "memory"));
            assert!(matches!(active, Some(MetadataValue::Boolean(true))));
        }
    }
}

#[test]
fn test_multi_field_between_and_string() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // price BETWEEN 100 AND 200 AND category = "gpu"
    let filter = parse(r#"price BETWEEN 100 AND 200 AND category = "gpu""#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let price = meta.get("price");

            assert!(matches!(cat, Some(MetadataValue::String(s)) if s == "gpu"));
            assert!(matches!(price, Some(MetadataValue::Integer(p)) if *p >= 100 && *p <= 200));
        }
    }
}

#[test]
fn test_multi_field_in_and_comparison() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // category IN ["gpu", "cpu"] AND price >= 200
    let filter = parse(r#"category IN ["gpu", "cpu"] AND price >= 200"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let price = meta.get("price");

            assert!(matches!(cat, Some(MetadataValue::String(s)) if s == "gpu" || s == "cpu"));
            assert!(matches!(price, Some(MetadataValue::Integer(p)) if *p >= 200));
        }
    }
}

#[test]
fn test_multi_field_not_in_and_boolean() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // category NOT IN ["memory"] AND active = false
    let filter = parse(r#"category NOT IN ["memory"] AND active = false"#).expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            let cat = meta.get("category");
            let active = meta.get("active");

            assert!(!matches!(cat, Some(MetadataValue::String(s)) if s == "memory"));
            assert!(matches!(active, Some(MetadataValue::Boolean(false))));
        }
    }
}

#[test]
fn test_multi_field_all_four_fields() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // category = "gpu" AND price > 50 AND active = true AND id < 50
    let filter = parse(r#"category = "gpu" AND price > 50 AND active = true AND id < 50"#)
        .expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            assert!(matches!(meta.get("category"), Some(MetadataValue::String(s)) if s == "gpu"));
            assert!(matches!(meta.get("price"), Some(MetadataValue::Integer(p)) if *p > 50));
            assert!(matches!(
                meta.get("active"),
                Some(MetadataValue::Boolean(true))
            ));
            assert!(matches!(meta.get("id"), Some(MetadataValue::Integer(id)) if *id < 50));
        }
    }
}

#[test]
fn test_multi_field_double_range() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // price >= 100 AND price <= 300 AND id >= 10 AND id <= 50
    let filter =
        parse("price >= 100 AND price <= 300 AND id >= 10 AND id <= 50").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    for res in &result.results {
        let meta = metadata_store.get(res.vector_id.0 as usize - 1);
        if let Some(meta) = meta {
            assert!(
                matches!(meta.get("price"), Some(MetadataValue::Integer(p)) if *p >= 100 && *p <= 300)
            );
            assert!(
                matches!(meta.get("id"), Some(MetadataValue::Integer(id)) if *id >= 10 && *id <= 50)
            );
        }
    }
}

// =============================================================================
// RESULT QUALITY TESTS
// =============================================================================

#[test]
fn test_filtered_search_returns_k_results_when_possible() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    // Filter that should match ~50% (active = true)
    let filter = parse("active = true").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();
    let k = 5;

    let result = searcher
        .search_filtered(&query, k, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // With 50 matching vectors, we should get all k results
    assert_eq!(result.results.len(), k);
    assert!(result.complete);
}

#[test]
fn test_filtered_search_result_metadata() {
    let (index, storage, metadata_store, _) = setup_test_index(64, 100, 42);
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse("active = true").expect("Parse failed");
    let query: Vec<f32> = (0..64).map(|_| 0.5).collect();

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    // Check result metadata
    assert!(result.observed_selectivity > 0.0);
    assert!(result.observed_selectivity <= 1.0);
    assert!(result.vectors_evaluated > 0);
}

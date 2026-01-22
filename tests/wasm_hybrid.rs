//! WASM Integration Tests for Sparse/Hybrid Search (Week 39)
//!
//! Tests the WASM bindings for sparse storage and hybrid search.
//! Run with: wasm-pack test --headless --chrome --features sparse

use edgevec::wasm::init_logging;
use edgevec::wasm::{EdgeVec, EdgeVecConfig};
use js_sys::{Float32Array, Uint32Array};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// =============================================================================
// SPARSE STORAGE INITIALIZATION TESTS
// =============================================================================

#[wasm_bindgen_test]
fn test_sparse_storage_initialization() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();

    // Initially no sparse storage
    assert!(!index.has_sparse_storage());
    assert_eq!(index.sparse_count(), 0);

    // Initialize sparse storage
    index.init_sparse_storage();

    // Now sparse storage should be available
    assert!(index.has_sparse_storage());
    assert_eq!(index.sparse_count(), 0);
}

#[wasm_bindgen_test]
fn test_sparse_storage_double_init() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();

    // Double init should be safe (no-op)
    index.init_sparse_storage();
    index.init_sparse_storage();

    assert!(index.has_sparse_storage());
}

// =============================================================================
// SPARSE INSERT TESTS
// =============================================================================

#[wasm_bindgen_test]
fn test_insert_sparse_basic() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Insert a sparse vector
    let indices = Uint32Array::from(&[0u32, 5, 10][..]);
    let values = Float32Array::from(&[1.0f32, 2.0, 3.0][..]);

    let id = index.insert_sparse(indices, values, 100).unwrap();

    assert!(id >= 0.0, "ID should be non-negative");
    assert_eq!(index.sparse_count(), 1);
}

#[wasm_bindgen_test]
fn test_insert_sparse_multiple() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Insert multiple sparse vectors
    for i in 0..10 {
        let indices = Uint32Array::from(&[i as u32, i as u32 + 10, i as u32 + 20][..]);
        let values = Float32Array::from(&[1.0f32, 2.0, 3.0][..]);
        let _id = index.insert_sparse(indices, values, 100).unwrap();
    }

    assert_eq!(index.sparse_count(), 10);
}

#[wasm_bindgen_test]
fn test_insert_sparse_length_mismatch() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Indices and values have different lengths
    let indices = Uint32Array::from(&[0u32, 5, 10][..]);
    let values = Float32Array::from(&[1.0f32, 2.0][..]); // Only 2 values

    let result = index.insert_sparse(indices, values, 100);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_str = err.as_string().unwrap_or_default();
    assert!(
        err_str.contains("same length"),
        "Error should mention length mismatch"
    );
}

#[wasm_bindgen_test]
fn test_insert_sparse_unsorted_indices() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Indices not sorted ascending
    let indices = Uint32Array::from(&[10u32, 5, 0][..]); // Wrong order
    let values = Float32Array::from(&[1.0f32, 2.0, 3.0][..]);

    let result = index.insert_sparse(indices, values, 100);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_insert_sparse_auto_init() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();

    // Don't explicitly init sparse storage - should auto-init on insert
    let indices = Uint32Array::from(&[0u32, 5, 10][..]);
    let values = Float32Array::from(&[1.0f32, 2.0, 3.0][..]);

    let result = index.insert_sparse(indices, values, 100);
    assert!(result.is_ok(), "Should auto-init sparse storage on insert");

    assert!(index.has_sparse_storage());
    assert_eq!(index.sparse_count(), 1);
}

// =============================================================================
// SPARSE SEARCH TESTS
// =============================================================================

#[wasm_bindgen_test]
fn test_search_sparse_basic() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Insert sparse vectors
    let indices1 = Uint32Array::from(&[0u32, 5, 10][..]);
    let values1 = Float32Array::from(&[1.0f32, 2.0, 3.0][..]);
    index.insert_sparse(indices1, values1, 100).unwrap();

    let indices2 = Uint32Array::from(&[5u32, 15, 25][..]);
    let values2 = Float32Array::from(&[4.0f32, 5.0, 6.0][..]);
    index.insert_sparse(indices2, values2, 100).unwrap();

    // Search - query overlaps with first vector at index 5
    let query_indices = Uint32Array::from(&[5u32, 10][..]);
    let query_values = Float32Array::from(&[1.0f32, 1.0][..]);

    let results_json = index
        .search_sparse(query_indices, query_values, 100, 10)
        .unwrap();

    // Parse JSON and verify results
    let results: Vec<serde_json::Value> = serde_json::from_str(&results_json).unwrap();
    assert!(results.len() <= 10, "Should return at most k results");
    assert!(!results.is_empty(), "Should find at least one match");

    // First result should have id and score
    let first = &results[0];
    assert!(first.get("id").is_some(), "Result should have id");
    assert!(first.get("score").is_some(), "Result should have score");
}

#[wasm_bindgen_test]
fn test_search_sparse_no_init_error() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let index = EdgeVec::new(&config).unwrap();

    // Don't init sparse storage
    let query_indices = Uint32Array::from(&[0u32, 5][..]);
    let query_values = Float32Array::from(&[1.0f32, 1.0][..]);

    let result = index.search_sparse(query_indices, query_values, 100, 10);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_str = err.as_string().unwrap_or_default();
    assert!(
        err_str.contains("not initialized"),
        "Error should mention sparse storage not initialized"
    );
}

#[wasm_bindgen_test]
fn test_search_sparse_k_zero_error() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    let indices = Uint32Array::from(&[0u32, 5][..]);
    let values = Float32Array::from(&[1.0f32, 1.0][..]);
    index
        .insert_sparse(indices.clone(), values.clone(), 100)
        .unwrap();

    // k = 0 should error
    let result = index.search_sparse(indices, values, 100, 0);
    assert!(result.is_err());
}

// =============================================================================
// HYBRID SEARCH TESTS
// =============================================================================

#[wasm_bindgen_test]
fn test_hybrid_search_basic() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Insert dense vector
    let dense_data = Float32Array::from(&[1.0f32, 0.0, 0.0][..]);
    let _dense_id = index.insert(dense_data).unwrap();

    // Insert sparse vector (aligned by insert order)
    let sparse_indices = Uint32Array::from(&[0u32, 5, 10][..]);
    let sparse_values = Float32Array::from(&[1.0f32, 2.0, 3.0][..]);
    let _sparse_id = index
        .insert_sparse(sparse_indices, sparse_values, 100)
        .unwrap();

    // Hybrid search
    let dense_query = Float32Array::from(&[1.0f32, 0.0, 0.0][..]);
    let sparse_query_indices = Uint32Array::from(&[0u32, 5][..]);
    let sparse_query_values = Float32Array::from(&[1.0f32, 1.0][..]);

    let options = r#"{"k": 5, "fusion": "rrf"}"#;

    let results_json = index
        .hybrid_search(
            dense_query,
            sparse_query_indices,
            sparse_query_values,
            100,
            options,
        )
        .unwrap();

    // Parse JSON
    let results: Vec<serde_json::Value> = serde_json::from_str(&results_json).unwrap();
    assert!(!results.is_empty(), "Should return results");

    // Verify result structure
    let first = &results[0];
    assert!(first.get("id").is_some(), "Result should have id");
    assert!(first.get("score").is_some(), "Result should have score");
}

#[wasm_bindgen_test]
fn test_hybrid_search_linear_fusion() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Insert aligned dense + sparse vectors
    for i in 0..5 {
        let dense = Float32Array::from(&[i as f32 / 5.0, 1.0 - i as f32 / 5.0, 0.0][..]);
        let _did = index.insert(dense).unwrap();

        let sparse_indices = Uint32Array::from(&[i as u32, 50 + i as u32][..]);
        let sparse_values = Float32Array::from(&[1.0f32, 2.0][..]);
        let _sid = index
            .insert_sparse(sparse_indices, sparse_values, 100)
            .unwrap();
    }

    // Hybrid search with linear fusion
    let dense_query = Float32Array::from(&[0.5f32, 0.5, 0.0][..]);
    let sparse_query_indices = Uint32Array::from(&[2u32, 52][..]);
    let sparse_query_values = Float32Array::from(&[1.0f32, 1.0][..]);

    let options = r#"{"k": 3, "fusion": {"type": "linear", "alpha": 0.7}}"#;

    let results_json = index
        .hybrid_search(
            dense_query,
            sparse_query_indices,
            sparse_query_values,
            100,
            options,
        )
        .unwrap();

    let results: Vec<serde_json::Value> = serde_json::from_str(&results_json).unwrap();
    assert_eq!(results.len(), 3, "Should return k=3 results");

    // Results should be sorted by score (descending)
    for i in 1..results.len() {
        let prev_score = results[i - 1]["score"].as_f64().unwrap();
        let curr_score = results[i]["score"].as_f64().unwrap();
        assert!(
            prev_score >= curr_score,
            "Results should be sorted by descending score"
        );
    }
}

#[wasm_bindgen_test]
fn test_hybrid_search_dimension_mismatch() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Wrong dense query dimensions
    let dense_query = Float32Array::from(&[1.0f32, 0.0][..]); // Only 2 dims
    let sparse_query_indices = Uint32Array::from(&[0u32][..]);
    let sparse_query_values = Float32Array::from(&[1.0f32][..]);

    let options = r#"{"k": 5, "fusion": "rrf"}"#;

    let result = index.hybrid_search(
        dense_query,
        sparse_query_indices,
        sparse_query_values,
        100,
        options,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.as_string().unwrap_or_default();
    assert!(
        err_str.contains("dimension") || err_str.contains("mismatch"),
        "Error should mention dimension mismatch"
    );
}

#[wasm_bindgen_test]
fn test_hybrid_search_invalid_options() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    let dense_query = Float32Array::from(&[1.0f32, 0.0, 0.0][..]);
    let sparse_query_indices = Uint32Array::from(&[0u32][..]);
    let sparse_query_values = Float32Array::from(&[1.0f32][..]);

    // Invalid JSON
    let options = r#"{"k": invalid}"#;

    let result = index.hybrid_search(
        dense_query,
        sparse_query_indices,
        sparse_query_values,
        100,
        options,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.as_string().unwrap_or_default();
    assert!(
        err_str.contains("Invalid") || err_str.contains("JSON"),
        "Error should mention invalid JSON"
    );
}

#[wasm_bindgen_test]
fn test_hybrid_search_sparse_not_init() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let index = EdgeVec::new(&config).unwrap();
    // Don't init sparse storage

    let dense_query = Float32Array::from(&[1.0f32, 0.0, 0.0][..]);
    let sparse_query_indices = Uint32Array::from(&[0u32][..]);
    let sparse_query_values = Float32Array::from(&[1.0f32][..]);

    let options = r#"{"k": 5, "fusion": "rrf"}"#;

    let result = index.hybrid_search(
        dense_query,
        sparse_query_indices,
        sparse_query_values,
        100,
        options,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.as_string().unwrap_or_default();
    assert!(
        err_str.contains("not initialized"),
        "Error should mention sparse storage not initialized"
    );
}

// =============================================================================
// HYBRID SEARCH WITH SCORES TESTS
// =============================================================================

#[wasm_bindgen_test]
fn test_hybrid_search_result_structure() {
    init_logging();

    let config = EdgeVecConfig::new(4);
    let mut index = EdgeVec::new(&config).unwrap();
    index.init_sparse_storage();

    // Insert multiple aligned vectors
    for i in 0..10 {
        let val = i as f32 / 10.0;
        let dense = Float32Array::from(&[val, val, 1.0 - val, 0.5][..]);
        let _did = index.insert(dense).unwrap();

        let sparse_indices = Uint32Array::from(&[(i * 10) as u32, (i * 10 + 5) as u32][..]);
        let sparse_values = Float32Array::from(&[1.0 + val, 2.0 - val][..]);
        let _sid = index
            .insert_sparse(sparse_indices, sparse_values, 1000)
            .unwrap();
    }

    // Search that should hit both dense and sparse results
    let dense_query = Float32Array::from(&[0.5f32, 0.5, 0.5, 0.5][..]);
    let sparse_query_indices = Uint32Array::from(&[50u32, 55][..]);
    let sparse_query_values = Float32Array::from(&[1.0f32, 1.0][..]);

    let options = r#"{"k": 5, "dense_k": 10, "sparse_k": 10, "fusion": "rrf"}"#;

    let results_json = index
        .hybrid_search(
            dense_query,
            sparse_query_indices,
            sparse_query_values,
            1000,
            options,
        )
        .unwrap();

    let results: Vec<serde_json::Value> = serde_json::from_str(&results_json).unwrap();
    assert_eq!(results.len(), 5, "Should return k=5 results");

    // Check that results have the expected structure
    for result in &results {
        // Required fields
        assert!(result.get("id").is_some(), "Result must have id");
        assert!(result.get("score").is_some(), "Result must have score");

        // Optional fields (may be present if result came from that source)
        // dense_rank, dense_score, sparse_rank, sparse_score
        // At least one source should be present
        let has_dense = result.get("dense_rank").is_some();
        let has_sparse = result.get("sparse_rank").is_some();
        assert!(
            has_dense || has_sparse,
            "Result should come from at least one source"
        );
    }
}

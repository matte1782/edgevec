//! Integration tests for HybridSearcher.
//!
//! Tests the full hybrid search pipeline: dense (HNSW) + sparse + fusion.

use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::hybrid::{FusionMethod, HybridSearchConfig, HybridSearcher};
use edgevec::sparse::{SparseStorage, SparseVector};
use edgevec::storage::VectorStorage;

/// Create a test setup with aligned dense and sparse vectors.
///
/// Returns (HnswIndex, VectorStorage, SparseStorage) with matching IDs.
fn create_test_setup(num_vectors: usize, dim: usize) -> (HnswIndex, VectorStorage, SparseStorage) {
    let config = HnswConfig::new(dim as u32);
    let mut dense_storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &dense_storage).expect("failed to create index");
    let mut sparse_storage = SparseStorage::new();

    for i in 0..num_vectors {
        // Dense vector: gradient from 0 to 1 based on index
        let val = i as f32 / num_vectors as f32;
        let dense: Vec<f32> = vec![val; dim];
        let _id = index
            .insert(&dense, &mut dense_storage)
            .expect("insert failed");

        // Sparse vector: unique indices based on i
        // Each vector has 5 non-zero entries at indices [i*10, i*10+1, ...]
        let sparse_indices: Vec<u32> = (0..5).map(|j| (i * 10 + j) as u32).collect();
        let sparse_values: Vec<f32> = vec![1.0; 5];
        let sparse = SparseVector::new(sparse_indices, sparse_values, 10000).unwrap();
        sparse_storage
            .insert(&sparse)
            .expect("sparse insert failed");
    }

    (index, dense_storage, sparse_storage)
}

#[test]
fn test_hybrid_search_basic() {
    let (index, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    // Query: middle-ish dense + sparse that matches vector 50
    let dense_query: Vec<f32> = vec![0.5; 64];
    let sparse_query = SparseVector::new(
        vec![500, 501, 502, 503, 504], // Matches vector 50
        vec![1.0; 5],
        10000,
    )
    .unwrap();

    let config = HybridSearchConfig::rrf(20, 20, 10);
    let results = searcher
        .search(&dense_query, &sparse_query, &config)
        .unwrap();

    assert_eq!(results.len(), 10);

    // All results should have valid IDs (< 100)
    for result in &results {
        assert!(result.id.0 < 100, "Invalid ID: {}", result.id.0);
    }
}

#[test]
fn test_hybrid_search_rrf_fusion() {
    let (index, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    // Query that matches vector 0 in sparse, but middle range (~50) in dense
    let dense_query: Vec<f32> = vec![0.5; 64];
    let sparse_query = SparseVector::new(
        vec![0, 1, 2, 3, 4], // Matches vector 0
        vec![1.0; 5],
        10000,
    )
    .unwrap();

    let config = HybridSearchConfig::rrf(20, 20, 10);
    let results = searcher
        .search(&dense_query, &sparse_query, &config)
        .unwrap();

    let ids: Vec<u64> = results.iter().map(|r| r.id.0).collect();

    // Vector 0 should appear in results due to strong sparse match
    assert!(ids.contains(&0), "Sparse match (ID 0) should be in results");
}

#[test]
fn test_hybrid_search_linear_fusion() {
    let (index, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    let dense_query: Vec<f32> = vec![0.5; 64];
    let sparse_query = SparseVector::new(vec![500, 501, 502], vec![1.0; 3], 10000).unwrap();

    let config = HybridSearchConfig::linear(20, 20, 10, 0.5).unwrap();
    let results = searcher
        .search(&dense_query, &sparse_query, &config)
        .unwrap();

    assert_eq!(results.len(), 10);

    // Verify results are sorted by descending score
    for i in 1..results.len() {
        assert!(
            results[i - 1].score >= results[i].score,
            "Results not sorted at index {}",
            i
        );
    }
}

#[test]
fn test_hybrid_search_dense_only() {
    let (index, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    let dense_query: Vec<f32> = vec![0.5; 64];
    let results = searcher.search_dense_only(&dense_query, 10).unwrap();

    assert_eq!(results.len(), 10);

    // All results should have dense_rank but NO sparse_rank.
    // search_dense_only sets sparse_k=0, so no sparse search runs.
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.dense_rank.is_some(),
            "Result {} should have dense_rank",
            i
        );
        assert!(
            result.sparse_rank.is_none(),
            "Result {} should have sparse_rank=None in dense-only mode, got {:?}",
            i,
            result.sparse_rank
        );
        assert!(
            result.sparse_score.is_none(),
            "Result {} should have sparse_score=None in dense-only mode, got {:?}",
            i,
            result.sparse_score
        );
        assert!(
            result.dense_score.is_some(),
            "Result {} should have dense_score in dense-only mode",
            i
        );
    }

    // Results should be sorted by descending score
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "Dense-only results not sorted: {} vs {}",
            window[0].score,
            window[1].score
        );
    }
}

#[test]
fn test_hybrid_search_sparse_only() {
    let (index, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    // Query that matches vector 0
    let sparse_query = SparseVector::new(vec![0, 1, 2, 3, 4], vec![1.0; 5], 10000).unwrap();

    let results = searcher.search_sparse_only(&sparse_query, 10).unwrap();

    // Should find at least some results
    assert!(!results.is_empty(), "Sparse-only should find results");

    // First result should be vector 0 (exact match)
    assert_eq!(
        results[0].id.0, 0,
        "First sparse result should be exact match"
    );
}

#[test]
fn test_hybrid_search_no_sparse_matches() {
    let (index, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    let dense_query: Vec<f32> = vec![0.5; 64];
    // Query with indices that don't exist in any vector
    let sparse_query = SparseVector::new(vec![9999], vec![1.0], 10000).unwrap();

    let config = HybridSearchConfig::rrf(20, 20, 10);
    let results = searcher
        .search(&dense_query, &sparse_query, &config)
        .unwrap();

    // Should still return results (from dense search)
    assert!(!results.is_empty(), "Should return dense results");

    // All results should come from dense only
    for result in &results {
        assert!(result.dense_rank.is_some(), "Should have dense rank");
        assert!(result.sparse_rank.is_none(), "Should not have sparse rank");
    }
}

#[test]
fn test_hybrid_search_result_includes_scores() {
    let (index, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    let dense_query: Vec<f32> = vec![0.5; 64];
    let sparse_query =
        SparseVector::new(vec![500, 501, 502, 503, 504], vec![1.0; 5], 10000).unwrap();

    let config = HybridSearchConfig::rrf(20, 20, 10);
    let results = searcher
        .search(&dense_query, &sparse_query, &config)
        .unwrap();

    // Check that results with ranks have corresponding scores
    for result in &results {
        if result.dense_rank.is_some() {
            assert!(
                result.dense_score.is_some(),
                "Result with dense_rank should have dense_score"
            );
        }
        if result.sparse_rank.is_some() {
            assert!(
                result.sparse_score.is_some(),
                "Result with sparse_rank should have sparse_score"
            );
        }
    }
}

#[test]
fn test_hybrid_config_validation() {
    // Both zero - invalid
    let config = HybridSearchConfig {
        dense_k: 0,
        sparse_k: 0,
        final_k: 10,
        fusion: FusionMethod::rrf(),
    };
    assert!(config.validate().is_err());

    // final_k zero - invalid
    let config = HybridSearchConfig {
        dense_k: 10,
        sparse_k: 10,
        final_k: 0,
        fusion: FusionMethod::rrf(),
    };
    assert!(config.validate().is_err());

    // Default - valid
    let config = HybridSearchConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_hybrid_dense_only_via_config() {
    let (index, dense_storage, sparse_storage) = create_test_setup(50, 32);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    // Dense-only via config (sparse_k = 0)
    let config = HybridSearchConfig {
        dense_k: 10,
        sparse_k: 0,
        final_k: 5,
        fusion: FusionMethod::rrf(),
    };

    let dense_query: Vec<f32> = vec![0.5; 32];
    let sparse_query = SparseVector::new(vec![0], vec![1.0], 100).unwrap();

    let results = searcher
        .search(&dense_query, &sparse_query, &config)
        .unwrap();

    assert_eq!(results.len(), 5);
}

#[test]
fn test_hybrid_sparse_only_via_config() {
    let (index, dense_storage, sparse_storage) = create_test_setup(50, 32);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    // Sparse-only via config (dense_k = 0)
    let config = HybridSearchConfig {
        dense_k: 0,
        sparse_k: 10,
        final_k: 5,
        fusion: FusionMethod::rrf(),
    };

    let dense_query: Vec<f32> = vec![0.5; 32];
    let sparse_query = SparseVector::new(vec![0, 1, 2, 3, 4], vec![1.0; 5], 10000).unwrap();

    let results = searcher
        .search(&dense_query, &sparse_query, &config)
        .unwrap();

    // Should have results (only from sparse)
    assert!(!results.is_empty());
}

#[test]
fn test_hybrid_searcher_counts() {
    let (index, dense_storage, sparse_storage) = create_test_setup(75, 16);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    assert_eq!(searcher.dense_count(), 75);
    assert_eq!(searcher.sparse_count(), 75);
}

#[test]
fn test_hybrid_searcher_components() {
    let (index, dense_storage, sparse_storage) = create_test_setup(10, 8);
    let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);

    let (idx, dense, sparse) = searcher.components();
    assert_eq!(dense.len(), 10);
    assert_eq!(sparse.len(), 10);
    assert_eq!(idx.len(), 10);
}

# Week 39 Day 3: HybridSearcher API

**Date:** 2026-01-28
**Focus:** Create unified `HybridSearcher` combining dense + sparse search
**Estimated Duration:** 3 hours
**Phase:** RFC-007 Implementation Phase 3 (Hybrid Search)
**Dependencies:** Day 1 (SparseSearcher), Day 2 (RRF Fusion)

---

## Context

Day 3 builds the `HybridSearcher` that orchestrates both dense (HNSW) and sparse searches, then fuses results. This is the main user-facing API for hybrid search.

**From ROADMAP v6.1 API Design:**

```typescript
const hybridResults = await db.hybridSearch({
  dense: { vector: embedding, k: 20 },
  sparse: { vector: bm25Scores, k: 20 },
  fusion: 'rrf',  // or { type: 'linear', alpha: 0.7 }
  k: 10
});
```

**Dependencies Available:**
- `SparseSearcher::search_u64()` from Day 1
- `rrf_fusion()` from Day 2
- `HnswGraph::search()` from existing codebase

---

## Tasks

### W39.3.1: Create `HybridSearchConfig` Struct

**Objective:** Define configuration for hybrid search.

**File:** `src/hybrid/search.rs`

```rust
//! Hybrid search combining dense and sparse retrieval.
//!
//! This module provides the `HybridSearcher` that orchestrates HNSW graph
//! search and sparse storage search, then fuses results using RRF or
//! linear combination.

use crate::error::GraphError;
use crate::hnsw::graph::HnswGraph;
use crate::hnsw::search::SearchResult;
use crate::sparse::{SparseSearcher, SparseStorage, SparseVector};
use crate::storage::VectorStorage;
use crate::types::VectorId;

use super::fusion::{rrf_fusion, linear_fusion, FusionMethod, FusionResult, RRF_DEFAULT_K};

/// Configuration for hybrid search.
///
/// Controls how many results to retrieve from each search type
/// and how to fuse them.
///
/// # Example
///
/// ```rust
/// use edgevec::hybrid::{HybridSearchConfig, FusionMethod};
///
/// // Retrieve 20 from each, return top 10 with RRF
/// let config = HybridSearchConfig {
///     dense_k: 20,
///     sparse_k: 20,
///     final_k: 10,
///     fusion: FusionMethod::rrf(),
/// };
///
/// // Or with linear combination
/// let config = HybridSearchConfig {
///     dense_k: 50,
///     sparse_k: 50,
///     final_k: 10,
///     fusion: FusionMethod::linear(0.7), // 70% dense, 30% sparse
/// };
/// ```
#[derive(Clone, Debug)]
pub struct HybridSearchConfig {
    /// Number of results to retrieve from dense (HNSW) search.
    /// More candidates improve recall but increase latency.
    pub dense_k: usize,

    /// Number of results to retrieve from sparse search.
    /// More candidates improve recall but increase latency.
    pub sparse_k: usize,

    /// Final number of results to return after fusion.
    pub final_k: usize,

    /// Fusion method to combine results.
    pub fusion: FusionMethod,
}

impl Default for HybridSearchConfig {
    /// Default configuration: 20 from each, return 10, RRF fusion.
    fn default() -> Self {
        Self {
            dense_k: 20,
            sparse_k: 20,
            final_k: 10,
            fusion: FusionMethod::default(),
        }
    }
}

impl HybridSearchConfig {
    /// Create a new hybrid search configuration.
    ///
    /// # Arguments
    ///
    /// * `dense_k` - Number of dense results to retrieve
    /// * `sparse_k` - Number of sparse results to retrieve
    /// * `final_k` - Final number of results
    /// * `fusion` - Fusion method
    #[must_use]
    pub fn new(dense_k: usize, sparse_k: usize, final_k: usize, fusion: FusionMethod) -> Self {
        Self {
            dense_k,
            sparse_k,
            final_k,
            fusion,
        }
    }

    /// Create config with RRF fusion (k=60).
    #[must_use]
    pub fn rrf(dense_k: usize, sparse_k: usize, final_k: usize) -> Self {
        Self::new(dense_k, sparse_k, final_k, FusionMethod::rrf())
    }

    /// Create config with RRF fusion using custom k.
    #[must_use]
    pub fn rrf_with_k(dense_k: usize, sparse_k: usize, final_k: usize, rrf_k: u32) -> Self {
        Self::new(dense_k, sparse_k, final_k, FusionMethod::rrf_with_k(rrf_k))
    }

    /// Create config with linear fusion.
    ///
    /// # Arguments
    ///
    /// * `alpha` - Weight for dense scores (0.0 = sparse only, 1.0 = dense only)
    #[must_use]
    pub fn linear(dense_k: usize, sparse_k: usize, final_k: usize, alpha: f32) -> Self {
        Self::new(dense_k, sparse_k, final_k, FusionMethod::linear(alpha))
    }

    /// Validate configuration.
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err` with description if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.dense_k == 0 && self.sparse_k == 0 {
            return Err("At least one of dense_k or sparse_k must be > 0".to_string());
        }
        if self.final_k == 0 {
            return Err("final_k must be > 0".to_string());
        }
        Ok(())
    }
}
```

**Acceptance Criteria:**
- [ ] `HybridSearchConfig` with all fields
- [ ] Default: 20/20/10 with RRF
- [ ] Factory methods: `rrf()`, `rrf_with_k()`, `linear()`
- [ ] `validate()` method for config validation
- [ ] Doc comments with examples

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W39.3.2: Create `HybridSearcher` Struct

**Objective:** Define the hybrid search orchestrator.

**File:** `src/hybrid/search.rs` (continued)

```rust
/// Result from hybrid search.
///
/// Contains the fused ranking with optional original ranks.
#[derive(Clone, Debug)]
pub struct HybridSearchResult {
    /// Vector ID
    pub id: VectorId,
    /// Combined score from fusion
    pub score: f32,
    /// Original rank in dense results (1-indexed, None if not found)
    pub dense_rank: Option<usize>,
    /// Original dense similarity score (None if not found)
    pub dense_score: Option<f32>,
    /// Original rank in sparse results (1-indexed, None if not found)
    pub sparse_rank: Option<usize>,
    /// Original sparse score (None if not found)
    pub sparse_score: Option<f32>,
}

impl HybridSearchResult {
    /// Create from fusion result.
    fn from_fusion(
        fusion: FusionResult,
        dense_results: &[(u64, f32)],
        sparse_results: &[(u64, f32)],
    ) -> Self {
        // Look up original scores
        let dense_score = dense_results
            .iter()
            .find(|(id, _)| *id == fusion.id)
            .map(|(_, score)| *score);

        let sparse_score = sparse_results
            .iter()
            .find(|(id, _)| *id == fusion.id)
            .map(|(_, score)| *score);

        Self {
            id: VectorId(fusion.id),
            score: fusion.score,
            dense_rank: fusion.dense_rank,
            dense_score,
            sparse_rank: fusion.sparse_rank,
            sparse_score,
        }
    }
}

/// Hybrid search combining dense and sparse retrieval.
///
/// Orchestrates HNSW graph search and sparse storage search,
/// then fuses results using the configured method.
///
/// # Example
///
/// ```rust,no_run
/// use edgevec::hybrid::{HybridSearcher, HybridSearchConfig};
/// use edgevec::sparse::SparseVector;
///
/// // Assume graph, dense_storage, sparse_storage are set up
/// # let graph = todo!();
/// # let dense_storage = todo!();
/// # let sparse_storage = todo!();
///
/// let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);
///
/// let dense_query = vec![0.1, 0.2, 0.3]; // embedding
/// let sparse_query = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100)?;
///
/// let config = HybridSearchConfig::rrf(20, 20, 10);
/// let results = searcher.search(&dense_query, &sparse_query, &config)?;
///
/// for result in &results {
///     println!("ID: {:?}, Score: {}", result.id, result.score);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct HybridSearcher<'a> {
    graph: &'a HnswGraph,
    dense_storage: &'a VectorStorage,
    sparse_storage: &'a SparseStorage,
}

impl<'a> HybridSearcher<'a> {
    /// Create a new hybrid searcher.
    ///
    /// # Arguments
    ///
    /// * `graph` - HNSW graph for dense search
    /// * `dense_storage` - Vector storage for dense vectors
    /// * `sparse_storage` - Sparse vector storage
    #[must_use]
    pub fn new(
        graph: &'a HnswGraph,
        dense_storage: &'a VectorStorage,
        sparse_storage: &'a SparseStorage,
    ) -> Self {
        Self {
            graph,
            dense_storage,
            sparse_storage,
        }
    }

    /// Perform hybrid search combining dense and sparse retrieval.
    ///
    /// # Algorithm
    ///
    /// 1. Execute dense search via HNSW graph
    /// 2. Execute sparse search via brute-force
    /// 3. Convert results to common ID format (u64)
    /// 4. Fuse results using configured method (RRF or Linear)
    /// 5. Return top-k fused results
    ///
    /// # Arguments
    ///
    /// * `dense_query` - Dense embedding vector for HNSW search
    /// * `sparse_query` - Sparse vector for keyword search
    /// * `config` - Hybrid search configuration
    ///
    /// # Returns
    ///
    /// Vec of `HybridSearchResult` sorted by fused score (descending).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Dense query dimension doesn't match graph
    /// - Config validation fails
    pub fn search(
        &self,
        dense_query: &[f32],
        sparse_query: &SparseVector,
        config: &HybridSearchConfig,
    ) -> Result<Vec<HybridSearchResult>, GraphError> {
        // Validate config
        config.validate().map_err(|e| GraphError::InvalidParameter(e))?;

        // Execute dense search
        let dense_results = if config.dense_k > 0 {
            self.graph
                .search(dense_query, config.dense_k, self.dense_storage)?
                .into_iter()
                .map(|r| (r.id.0, r.distance))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Execute sparse search
        let sparse_searcher = SparseSearcher::new(self.sparse_storage);
        let sparse_results = if config.sparse_k > 0 {
            sparse_searcher.search_u64(sparse_query, config.sparse_k)
        } else {
            Vec::new()
        };

        // Fuse results
        let fused = match &config.fusion {
            FusionMethod::Rrf { k } => {
                rrf_fusion(&dense_results, &sparse_results, *k, config.final_k)
            }
            FusionMethod::Linear { alpha } => {
                linear_fusion(&dense_results, &sparse_results, *alpha, config.final_k)
            }
        };

        // Convert to HybridSearchResult
        let results = fused
            .into_iter()
            .map(|f| HybridSearchResult::from_fusion(f, &dense_results, &sparse_results))
            .collect();

        Ok(results)
    }

    /// Search with dense-only (sparse disabled).
    ///
    /// Useful for A/B testing or when sparse features aren't available.
    pub fn search_dense_only(
        &self,
        dense_query: &[f32],
        k: usize,
    ) -> Result<Vec<HybridSearchResult>, GraphError> {
        let config = HybridSearchConfig {
            dense_k: k,
            sparse_k: 0,
            final_k: k,
            fusion: FusionMethod::rrf(), // Doesn't matter for dense-only
        };

        // Sparse query doesn't matter, use empty-ish vector
        let sparse_query = SparseVector::singleton(0, 0.0, 1)
            .map_err(|e| GraphError::InvalidParameter(e.to_string()))?;

        self.search(dense_query, &sparse_query, &config)
    }

    /// Search with sparse-only (dense disabled).
    ///
    /// Useful for keyword-only search or A/B testing.
    pub fn search_sparse_only(
        &self,
        sparse_query: &SparseVector,
        k: usize,
    ) -> Result<Vec<HybridSearchResult>, GraphError> {
        let config = HybridSearchConfig {
            dense_k: 0,
            sparse_k: k,
            final_k: k,
            fusion: FusionMethod::rrf(), // Doesn't matter for sparse-only
        };

        // Dense query doesn't matter, use zeros
        let dense_query = vec![0.0; self.dense_storage.dimensions()];

        self.search(&dense_query, sparse_query, &config)
    }

    /// Get references to underlying components.
    pub fn components(&self) -> (&HnswGraph, &VectorStorage, &SparseStorage) {
        (self.graph, self.dense_storage, self.sparse_storage)
    }
}
```

**Acceptance Criteria:**
- [ ] `HybridSearcher` struct with all storage references
- [ ] `search()` orchestrates dense + sparse + fusion
- [ ] `search_dense_only()` for dense-only mode
- [ ] `search_sparse_only()` for sparse-only mode
- [ ] `HybridSearchResult` includes original scores and ranks
- [ ] Error handling for dimension mismatch and invalid config
- [ ] Doc comments with examples

**Estimated Duration:** 1.5 hours

**Agent:** RUST_ENGINEER

---

### W39.3.3: Update Module Exports

**Objective:** Export `HybridSearcher` and related types.

**File:** `src/hybrid/mod.rs` update

```rust
//! Hybrid search combining dense and sparse retrieval.

mod fusion;
mod search;

pub use fusion::{
    rrf_fusion,
    linear_fusion,
    FusionResult,
    FusionMethod,
    RRF_DEFAULT_K,
};

pub use search::{
    HybridSearcher,
    HybridSearchConfig,
    HybridSearchResult,
};
```

**Acceptance Criteria:**
- [ ] All types exported
- [ ] `cargo check` passes
- [ ] Can use `HybridSearcher` from crate root

**Estimated Duration:** 10 minutes

**Agent:** RUST_ENGINEER

---

### W39.3.4: Integration Tests for HybridSearcher

**Objective:** End-to-end tests for hybrid search.

**File:** `tests/hybrid_search_test.rs`

```rust
//! Integration tests for hybrid search.

use edgevec::hybrid::{HybridSearcher, HybridSearchConfig, FusionMethod};
use edgevec::sparse::{SparseStorage, SparseVector};
use edgevec::hnsw::graph::{HnswGraph, HnswConfig};
use edgevec::storage::VectorStorage;
use edgevec::types::VectorId;

fn create_test_setup(num_vectors: usize, dim: usize) -> (HnswGraph, VectorStorage, SparseStorage) {
    // Create HNSW graph
    let config = HnswConfig {
        dimensions: dim as u32,
        ..HnswConfig::default()
    };
    let mut graph = HnswGraph::new(config);
    let mut dense_storage = VectorStorage::new(dim);

    // Create sparse storage
    let mut sparse_storage = SparseStorage::new();

    // Insert test vectors
    for i in 0..num_vectors {
        // Dense vector: [i/n, i/n, ...]
        let dense: Vec<f32> = (0..dim).map(|_| i as f32 / num_vectors as f32).collect();
        let id = dense_storage.insert(&dense).unwrap();
        graph.insert(id, &dense, &dense_storage).unwrap();

        // Sparse vector: indices based on i, values = 1.0
        let sparse_indices: Vec<u32> = (0..5).map(|j| (i * 10 + j) as u32).collect();
        let sparse_values: Vec<f32> = vec![1.0; 5];
        let sparse = SparseVector::new(sparse_indices, sparse_values, 10000).unwrap();
        sparse_storage.insert(&sparse).unwrap();
    }

    (graph, dense_storage, sparse_storage)
}

#[test]
fn test_hybrid_search_basic() {
    let (graph, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);

    // Query
    let dense_query: Vec<f32> = vec![0.5; 64]; // Middle of range
    let sparse_query = SparseVector::new(
        vec![500, 501, 502, 503, 504], // Matches vector 50
        vec![1.0; 5],
        10000,
    ).unwrap();

    let config = HybridSearchConfig::rrf(20, 20, 10);
    let results = searcher.search(&dense_query, &sparse_query, &config).unwrap();

    assert_eq!(results.len(), 10);

    // All results should have valid IDs
    for result in &results {
        assert!(result.id.0 < 100);
    }
}

#[test]
fn test_hybrid_search_rrf_fusion() {
    let (graph, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);

    // Query that matches vector 0 in sparse, vector ~50 in dense
    let dense_query: Vec<f32> = vec![0.5; 64];
    let sparse_query = SparseVector::new(
        vec![0, 1, 2, 3, 4], // Matches vector 0
        vec![1.0; 5],
        10000,
    ).unwrap();

    let config = HybridSearchConfig::rrf(20, 20, 10);
    let results = searcher.search(&dense_query, &sparse_query, &config).unwrap();

    // Results should include both sparse match (0) and dense matches (~50)
    let ids: Vec<u64> = results.iter().map(|r| r.id.0).collect();

    // Vector 0 should be in results due to sparse match
    assert!(ids.contains(&0), "Sparse match (ID 0) should be in results");
}

#[test]
fn test_hybrid_search_linear_fusion() {
    let (graph, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);

    let dense_query: Vec<f32> = vec![0.5; 64];
    let sparse_query = SparseVector::new(
        vec![500, 501, 502],
        vec![1.0; 3],
        10000,
    ).unwrap();

    let config = HybridSearchConfig::linear(20, 20, 10, 0.5);
    let results = searcher.search(&dense_query, &sparse_query, &config).unwrap();

    assert_eq!(results.len(), 10);
}

#[test]
fn test_hybrid_search_dense_only() {
    let (graph, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);

    let dense_query: Vec<f32> = vec![0.5; 64];
    let results = searcher.search_dense_only(&dense_query, 10).unwrap();

    assert_eq!(results.len(), 10);

    // All results should have dense rank
    for result in &results {
        assert!(result.dense_rank.is_some() || result.sparse_rank.is_none());
    }
}

#[test]
fn test_hybrid_search_sparse_only() {
    let (graph, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);

    let sparse_query = SparseVector::new(
        vec![0, 1, 2, 3, 4],
        vec![1.0; 5],
        10000,
    ).unwrap();

    let results = searcher.search_sparse_only(&sparse_query, 10).unwrap();

    // Should find at least the exact match
    assert!(!results.is_empty());

    // First result should be the sparse match
    assert_eq!(results[0].id.0, 0);
}

#[test]
fn test_hybrid_search_no_sparse_matches() {
    let (graph, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);

    let dense_query: Vec<f32> = vec![0.5; 64];
    // Query with indices that don't exist
    let sparse_query = SparseVector::new(
        vec![9999],
        vec![1.0],
        10000,
    ).unwrap();

    let config = HybridSearchConfig::rrf(20, 20, 10);
    let results = searcher.search(&dense_query, &sparse_query, &config).unwrap();

    // Should still return results (from dense)
    assert!(!results.is_empty());

    // All results should come from dense only
    for result in &results {
        assert!(result.dense_rank.is_some());
        assert!(result.sparse_rank.is_none());
    }
}

#[test]
fn test_hybrid_search_result_includes_scores() {
    let (graph, dense_storage, sparse_storage) = create_test_setup(100, 64);
    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);

    let dense_query: Vec<f32> = vec![0.5; 64];
    let sparse_query = SparseVector::new(
        vec![500, 501, 502, 503, 504],
        vec![1.0; 5],
        10000,
    ).unwrap();

    let config = HybridSearchConfig::rrf(20, 20, 10);
    let results = searcher.search(&dense_query, &sparse_query, &config).unwrap();

    // Check that results with dense_rank have dense_score
    for result in &results {
        if result.dense_rank.is_some() {
            assert!(result.dense_score.is_some());
        }
        if result.sparse_rank.is_some() {
            assert!(result.sparse_score.is_some());
        }
    }
}

#[test]
fn test_hybrid_config_validation() {
    let config = HybridSearchConfig {
        dense_k: 0,
        sparse_k: 0,
        final_k: 10,
        fusion: FusionMethod::rrf(),
    };

    assert!(config.validate().is_err());

    let config = HybridSearchConfig {
        dense_k: 10,
        sparse_k: 10,
        final_k: 0,
        fusion: FusionMethod::rrf(),
    };

    assert!(config.validate().is_err());

    let config = HybridSearchConfig::default();
    assert!(config.validate().is_ok());
}
```

**Acceptance Criteria:**
- [ ] `test_hybrid_search_basic` - End-to-end works
- [ ] `test_hybrid_search_rrf_fusion` - RRF mode works
- [ ] `test_hybrid_search_linear_fusion` - Linear mode works
- [ ] `test_hybrid_search_dense_only` - Dense-only mode
- [ ] `test_hybrid_search_sparse_only` - Sparse-only mode
- [ ] `test_hybrid_search_no_sparse_matches` - Graceful handling
- [ ] `test_hybrid_search_result_includes_scores` - Scores present
- [ ] `test_hybrid_config_validation` - Config validation
- [ ] All tests pass: `cargo test --features sparse hybrid_search_test`

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

## Day 3 Checklist

- [ ] W39.3.1: `HybridSearchConfig` struct with factory methods
- [ ] W39.3.2: `HybridSearcher` with `search()`, `search_dense_only()`, `search_sparse_only()`
- [ ] W39.3.3: Module exports updated
- [ ] W39.3.4: All 8+ integration tests passing
- [ ] `cargo check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test hybrid` passes
- [ ] `cargo test --test hybrid_search_test` passes

---

## Day 3 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `HybridSearcher` compiles | `cargo check` |
| Full search pipeline works | `test_hybrid_search_basic` |
| RRF fusion integrated | `test_hybrid_search_rrf_fusion` |
| Linear fusion integrated | `test_hybrid_search_linear_fusion` |
| Dense-only mode | `test_hybrid_search_dense_only` |
| Sparse-only mode | `test_hybrid_search_sparse_only` |
| Config validation | `test_hybrid_config_validation` |
| Clippy clean | `cargo clippy -- -D warnings` |

---

## Day 3 Handoff

After completing Day 3:

**Artifacts Generated:**
- `src/hybrid/search.rs` with `HybridSearcher` implementation
- Updated `src/hybrid/mod.rs` with exports
- `tests/hybrid_search_test.rs` integration tests

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 4 - Linear Combination Fusion Implementation

---

## Notes for Implementation

### ID Mapping [HOSTILE_REVIEW: M1 Resolution]

Dense vectors use `VectorId` and sparse vectors use `SparseId`. For fusion,
we convert both to `u64`.

**IMPORTANT: ID Alignment Contract**

The caller is responsible for ensuring that the same document has matching
numeric IDs in both dense and sparse storage:

1. **Insert Order Alignment**: Insert dense and sparse vectors in the same
   order, so auto-assigned IDs match (recommended for simple use cases).

2. **Explicit ID Management**: Use `insert_with_id()` methods (when available)
   to explicitly set matching IDs for the same document.

3. **Mapping Layer**: Maintain an external document_id → (VectorId, SparseId)
   mapping if IDs cannot be aligned at insert time.

**If IDs don't align**, fusion will treat them as different documents, and
hybrid search quality will degrade significantly.

**Future Enhancement (v0.10.0)**: Consider adding an optional ID mapping
parameter to `HybridSearcher` for cases where ID alignment cannot be
guaranteed at insert time.

**Implementation Note**: Add validation in `HybridSearcher::new()` to warn
if dense_storage.len() != sparse_storage.len() (suggests possible misalignment).

### Performance Considerations

The hybrid search pipeline:
1. Dense search: O(log n * ef_search) via HNSW
2. Sparse search: O(n * avg_nnz) brute force
3. Fusion: O(dense_k + sparse_k)

For 10k vectors, typical latency:
- Dense: ~1ms
- Sparse: ~5ms
- Fusion: <1ms
- Total: ~7ms

This is acceptable for v0.9.0.

### Error Handling

The `search()` method returns `GraphError` for consistency with existing
HNSW search. Sparse search errors are wrapped in `GraphError::InvalidParameter`.

---

*Agent: PLANNER*
*Status: [PROPOSED]*
*Date: 2026-01-21*

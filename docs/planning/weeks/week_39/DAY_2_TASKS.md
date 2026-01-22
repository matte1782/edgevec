# Week 39 Day 2: RRF Fusion Algorithm

**Date:** 2026-01-27
**Focus:** Implement Reciprocal Rank Fusion (RRF) algorithm
**Estimated Duration:** 3 hours
**Phase:** RFC-007 Implementation Phase 3 (Hybrid Search)
**Dependencies:** Day 1 COMPLETE (SparseSearcher)

---

## Context

Day 2 implements the RRF (Reciprocal Rank Fusion) algorithm that combines dense and sparse search results. RRF is the industry-standard fusion method used by Milvus, Weaviate, and pgvector.

**From ROADMAP v6.1:**

```typescript
// RRF Algorithm (RFC_BM25_HYBRID_SEARCH.md)
function rrf(dense: Id[], sparse: Id[], k = 60): Id[] {
  const scores = new Map<Id, number>();
  dense.forEach((id, rank) => {
    scores.set(id, (scores.get(id) || 0) + 1 / (k + rank + 1));
  });
  sparse.forEach((id, rank) => {
    scores.set(id, (scores.get(id) || 0) + 1 / (k + rank + 1));
  });
  return [...scores.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, k)
    .map(([id]) => id);
}
```

**Reference Paper:**
Cormack et al. "Reciprocal Rank Fusion outperforms Condorcet and individual Rank Learning Methods" (SIGIR 2009)

---

## Tasks

### W39.2.1: Create `src/hybrid/` Module Structure

**Objective:** Create new hybrid search module for fusion algorithms.

**Directory Structure:**
```
src/hybrid/
├── mod.rs              # Module exports
├── fusion.rs           # RRF + Linear fusion algorithms
└── search.rs           # HybridSearcher (Day 3)
```

**File:** `src/hybrid/mod.rs`

```rust
//! Hybrid search combining dense and sparse retrieval.
//!
//! This module provides fusion algorithms and search orchestration for
//! combining dense semantic search (HNSW) with sparse keyword search (BM25).
//!
//! # Fusion Methods
//!
//! Two fusion methods are supported:
//!
//! 1. **Reciprocal Rank Fusion (RRF)**: Combines ranks using
//!    `score(d) = sum(1 / (k + rank_i(d)))`. Robust, parameter-insensitive.
//!
//! 2. **Linear Combination**: Combines normalized scores using
//!    `score(d) = alpha * dense_score + (1 - alpha) * sparse_score`.
//!
//! # Example
//!
//! ```rust
//! use edgevec::hybrid::{rrf_fusion, FusionResult};
//!
//! // Dense search results: [(id, score), ...]
//! let dense = vec![(1, 0.95), (2, 0.80), (3, 0.75)];
//!
//! // Sparse search results: [(id, score), ...]
//! let sparse = vec![(2, 5.5), (4, 4.2), (1, 3.8)];
//!
//! // Fuse with RRF (k=60)
//! let fused = rrf_fusion(&dense, &sparse, 60, 10);
//!
//! // Results combine ranks from both lists
//! for result in &fused {
//!     println!("ID: {}, RRF Score: {}", result.id, result.score);
//! }
//! ```

mod fusion;

// HybridSearcher will be added in Day 3
// mod search;

pub use fusion::{
    rrf_fusion,
    linear_fusion,
    FusionResult,
    FusionMethod,
    RRF_DEFAULT_K,
};

// Will be added in Day 3:
// pub use search::HybridSearcher;
```

**File:** `src/lib.rs` addition

```rust
// Add to lib.rs
pub mod hybrid;
```

**Acceptance Criteria:**
- [ ] `src/hybrid/mod.rs` created
- [ ] `src/hybrid/fusion.rs` created (empty initially)
- [ ] `hybrid` module exported from `lib.rs`
- [ ] `cargo check` passes

**Estimated Duration:** 15 minutes

**Agent:** RUST_ENGINEER

---

### W39.2.2: Implement `FusionResult` and `FusionMethod` Types

**Objective:** Define common types for fusion algorithms.

**File:** `src/hybrid/fusion.rs`

```rust
//! Fusion algorithms for hybrid search.
//!
//! This module implements score fusion methods that combine results
//! from multiple retrieval systems (dense + sparse).

use std::collections::HashMap;

/// Default k parameter for RRF fusion.
///
/// k=60 is the standard value from the original RRF paper.
/// Higher values give more weight to documents ranked lower in lists.
pub const RRF_DEFAULT_K: u32 = 60;

/// Result from fusion algorithm.
#[derive(Clone, Debug)]
pub struct FusionResult {
    /// Document/vector ID
    pub id: u64,
    /// Combined score from fusion
    pub score: f32,
    /// Original rank in dense results (1-indexed, None if not present)
    pub dense_rank: Option<usize>,
    /// Original rank in sparse results (1-indexed, None if not present)
    pub sparse_rank: Option<usize>,
}

impl FusionResult {
    /// Create a new fusion result.
    #[inline]
    #[must_use]
    pub fn new(id: u64, score: f32) -> Self {
        Self {
            id,
            score,
            dense_rank: None,
            sparse_rank: None,
        }
    }

    /// Create with rank information.
    #[inline]
    #[must_use]
    pub fn with_ranks(
        id: u64,
        score: f32,
        dense_rank: Option<usize>,
        sparse_rank: Option<usize>,
    ) -> Self {
        Self {
            id,
            score,
            dense_rank,
            sparse_rank,
        }
    }
}

/// Fusion method configuration.
#[derive(Clone, Debug)]
pub enum FusionMethod {
    /// Reciprocal Rank Fusion with k parameter.
    ///
    /// Score = sum(1 / (k + rank_i)) across all lists.
    /// Default k=60 (industry standard).
    Rrf {
        /// The k parameter (default 60)
        k: u32,
    },

    /// Linear combination with alpha weight.
    ///
    /// Score = alpha * normalized_dense + (1 - alpha) * normalized_sparse.
    /// Alpha=0.5 gives equal weight, alpha=1.0 is dense-only.
    Linear {
        /// Weight for dense scores (0.0 to 1.0)
        alpha: f32,
    },
}

impl Default for FusionMethod {
    fn default() -> Self {
        FusionMethod::Rrf { k: RRF_DEFAULT_K }
    }
}

impl FusionMethod {
    /// Create RRF fusion with default k=60.
    #[inline]
    #[must_use]
    pub fn rrf() -> Self {
        FusionMethod::Rrf { k: RRF_DEFAULT_K }
    }

    /// Create RRF fusion with custom k parameter.
    #[inline]
    #[must_use]
    pub fn rrf_with_k(k: u32) -> Self {
        FusionMethod::Rrf { k }
    }

    /// Create linear fusion with alpha weight.
    ///
    /// # Panics
    ///
    /// Panics if alpha is not in range [0.0, 1.0].
    #[inline]
    #[must_use]
    pub fn linear(alpha: f32) -> Self {
        assert!(
            (0.0..=1.0).contains(&alpha),
            "Alpha must be in range [0.0, 1.0], got {}",
            alpha
        );
        FusionMethod::Linear { alpha }
    }

    /// Create balanced linear fusion (alpha=0.5).
    #[inline]
    #[must_use]
    pub fn linear_balanced() -> Self {
        FusionMethod::Linear { alpha: 0.5 }
    }
}
```

**Acceptance Criteria:**
- [ ] `FusionResult` struct with id, score, and optional ranks
- [ ] `FusionMethod` enum with `Rrf` and `Linear` variants
- [ ] Default implementation returns RRF with k=60
- [ ] Factory methods: `rrf()`, `rrf_with_k()`, `linear()`, `linear_balanced()`
- [ ] Doc comments with examples
- [ ] `RRF_DEFAULT_K` constant exported

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W39.2.3: Implement `rrf_fusion()` Function

**Objective:** Implement Reciprocal Rank Fusion algorithm.

**File:** `src/hybrid/fusion.rs` (continued)

```rust
/// Reciprocal Rank Fusion (RRF) algorithm.
///
/// Combines two ranked lists into a single ranking using the formula:
/// `score(d) = sum(1 / (k + rank_i(d)))` for each list `i`.
///
/// # Algorithm
///
/// 1. Build a map of document IDs to their ranks in each list
/// 2. For each unique document, compute RRF score
/// 3. Sort by descending RRF score
/// 4. Return top N results
///
/// # Arguments
///
/// * `dense_results` - Results from dense search as (id, score) tuples,
///   ordered by descending score. Scores are not used; only rank matters.
/// * `sparse_results` - Results from sparse search as (id, score) tuples,
///   ordered by descending score. Scores are not used; only rank matters.
/// * `k` - RRF smoothing parameter (default 60). Higher values give more
///   weight to documents that appear lower in the ranking.
/// * `top_n` - Number of results to return.
///
/// # Returns
///
/// Vec of `FusionResult` containing fused results sorted by descending
/// RRF score. Includes rank information from both lists.
///
/// # Performance
///
/// - Time: O(d + s + u log u) where d = |dense|, s = |sparse|, u = unique IDs
/// - Space: O(u) for the score map
///
/// # Reference
///
/// Cormack, G.V., Clarke, C.L.A., and Buettcher, S. (2009).
/// "Reciprocal Rank Fusion outperforms Condorcet and individual Rank Learning Methods"
/// SIGIR 2009.
///
/// # Example
///
/// ```rust
/// use edgevec::hybrid::rrf_fusion;
///
/// // Dense results (ordered by relevance)
/// let dense = vec![(1, 0.95), (2, 0.80), (3, 0.75)];
///
/// // Sparse results (ordered by BM25 score)
/// let sparse = vec![(2, 5.5), (4, 4.2), (1, 3.8)];
///
/// // Fuse with k=60
/// let results = rrf_fusion(&dense, &sparse, 60, 10);
///
/// // ID 2 appears in both lists (rank 2 + rank 1), should score high
/// // ID 1 appears in both lists (rank 1 + rank 3)
/// // ID 3 only in dense (rank 3)
/// // ID 4 only in sparse (rank 2)
/// assert_eq!(results[0].id, 2); // Best combined rank
/// ```
#[must_use]
pub fn rrf_fusion(
    dense_results: &[(u64, f32)],
    sparse_results: &[(u64, f32)],
    k: u32,
    top_n: usize,
) -> Vec<FusionResult> {
    if top_n == 0 {
        return Vec::new();
    }

    let k_f32 = k as f32;

    // Track scores and ranks for each document
    struct DocInfo {
        score: f32,
        dense_rank: Option<usize>,
        sparse_rank: Option<usize>,
    }

    let mut doc_map: HashMap<u64, DocInfo> = HashMap::new();

    // Process dense results (ranks are 1-indexed per RRF paper)
    for (rank_0, (id, _score)) in dense_results.iter().enumerate() {
        let rank = rank_0 + 1; // Convert to 1-indexed
        let rrf_contribution = 1.0 / (k_f32 + rank as f32);

        doc_map
            .entry(*id)
            .and_modify(|info| {
                info.score += rrf_contribution;
                info.dense_rank = Some(rank);
            })
            .or_insert(DocInfo {
                score: rrf_contribution,
                dense_rank: Some(rank),
                sparse_rank: None,
            });
    }

    // Process sparse results
    for (rank_0, (id, _score)) in sparse_results.iter().enumerate() {
        let rank = rank_0 + 1; // Convert to 1-indexed
        let rrf_contribution = 1.0 / (k_f32 + rank as f32);

        doc_map
            .entry(*id)
            .and_modify(|info| {
                info.score += rrf_contribution;
                info.sparse_rank = Some(rank);
            })
            .or_insert(DocInfo {
                score: rrf_contribution,
                dense_rank: None,
                sparse_rank: Some(rank),
            });
    }

    // Convert to results and sort
    let mut results: Vec<FusionResult> = doc_map
        .into_iter()
        .map(|(id, info)| FusionResult::with_ranks(
            id,
            info.score,
            info.dense_rank,
            info.sparse_rank,
        ))
        .collect();

    // Sort by descending score
    results.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Truncate to top_n
    results.truncate(top_n);

    results
}
```

**Acceptance Criteria:**
- [ ] Correct RRF formula: `1 / (k + rank)` where rank is 1-indexed
- [ ] Accumulates scores for documents in both lists
- [ ] Handles documents appearing in only one list
- [ ] Returns results sorted by descending RRF score
- [ ] Includes rank information from both sources
- [ ] Handles empty input lists
- [ ] Handles top_n = 0
- [ ] Doc comments with reference to paper

**Estimated Duration:** 1 hour

**Agent:** RUST_ENGINEER

---

### W39.2.4: Unit Tests for RRF Fusion

**Objective:** Comprehensive test coverage for RRF algorithm.

**File:** `src/hybrid/fusion.rs` (tests module)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ============= RRF Basic Tests =============

    #[test]
    fn test_rrf_identical_lists() {
        // Same results in both lists should give high scores
        let dense = vec![(1, 0.9), (2, 0.8), (3, 0.7)];
        let sparse = vec![(1, 5.0), (2, 4.0), (3, 3.0)];

        let results = rrf_fusion(&dense, &sparse, 60, 10);

        assert_eq!(results.len(), 3);
        // ID 1 is rank 1 in both lists: 2 * (1/61) = 0.0328
        // ID 2 is rank 2 in both lists: 2 * (1/62) = 0.0323
        // ID 3 is rank 3 in both lists: 2 * (1/63) = 0.0317
        assert_eq!(results[0].id, 1);
        assert_eq!(results[1].id, 2);
        assert_eq!(results[2].id, 3);

        // Verify scores are approximately equal (both rank 1)
        let expected_score = 2.0 / 61.0; // 1/(60+1) + 1/(60+1)
        assert!((results[0].score - expected_score).abs() < 1e-6);
    }

    #[test]
    fn test_rrf_disjoint_lists() {
        // No overlap between lists
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(3, 5.0), (4, 4.0)];

        let results = rrf_fusion(&dense, &sparse, 60, 10);

        assert_eq!(results.len(), 4);
        // All have same RRF contribution from single list
        // Rank 1: 1/61, Rank 2: 1/62
        for result in &results {
            assert!(result.dense_rank.is_some() != result.sparse_rank.is_some());
        }
    }

    #[test]
    fn test_rrf_partial_overlap() {
        // ID 2 appears in both, others in one
        let dense = vec![(1, 0.9), (2, 0.8), (3, 0.7)];
        let sparse = vec![(2, 5.0), (4, 4.0), (5, 3.0)];

        let results = rrf_fusion(&dense, &sparse, 60, 10);

        // ID 2 should be first (appears in both)
        assert_eq!(results[0].id, 2);
        assert_eq!(results[0].dense_rank, Some(2));
        assert_eq!(results[0].sparse_rank, Some(1));

        // Score for ID 2: 1/(60+2) + 1/(60+1) = 1/62 + 1/61
        let expected = 1.0 / 62.0 + 1.0 / 61.0;
        assert!((results[0].score - expected).abs() < 1e-6);
    }

    #[test]
    fn test_rrf_k_parameter_effect() {
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(1, 5.0), (2, 4.0)];

        // With k=60 (default)
        let results_k60 = rrf_fusion(&dense, &sparse, 60, 10);

        // With k=1 (more weight to top ranks)
        let results_k1 = rrf_fusion(&dense, &sparse, 1, 10);

        // With higher k, scores are smaller
        assert!(results_k60[0].score < results_k1[0].score);

        // But relative ordering should be same
        assert_eq!(results_k60[0].id, results_k1[0].id);
    }

    #[test]
    fn test_rrf_empty_dense() {
        let dense: Vec<(u64, f32)> = vec![];
        let sparse = vec![(1, 5.0), (2, 4.0)];

        let results = rrf_fusion(&dense, &sparse, 60, 10);

        assert_eq!(results.len(), 2);
        assert!(results[0].dense_rank.is_none());
        assert!(results[0].sparse_rank.is_some());
    }

    #[test]
    fn test_rrf_empty_sparse() {
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse: Vec<(u64, f32)> = vec![];

        let results = rrf_fusion(&dense, &sparse, 60, 10);

        assert_eq!(results.len(), 2);
        assert!(results[0].dense_rank.is_some());
        assert!(results[0].sparse_rank.is_none());
    }

    #[test]
    fn test_rrf_both_empty() {
        let dense: Vec<(u64, f32)> = vec![];
        let sparse: Vec<(u64, f32)> = vec![];

        let results = rrf_fusion(&dense, &sparse, 60, 10);

        assert!(results.is_empty());
    }

    #[test]
    fn test_rrf_top_n_zero() {
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(1, 5.0), (2, 4.0)];

        let results = rrf_fusion(&dense, &sparse, 60, 0);

        assert!(results.is_empty());
    }

    #[test]
    fn test_rrf_top_n_truncation() {
        let dense = vec![(1, 0.9), (2, 0.8), (3, 0.7)];
        let sparse = vec![(4, 5.0), (5, 4.0), (6, 3.0)];

        // Request only top 3, but 6 unique IDs exist
        let results = rrf_fusion(&dense, &sparse, 60, 3);

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_rrf_score_ordering() {
        // Test that results are actually sorted by descending score
        let dense = vec![(1, 0.9), (2, 0.8), (3, 0.7), (4, 0.6)];
        let sparse = vec![(4, 5.0), (3, 4.0), (2, 3.0), (1, 2.0)];

        let results = rrf_fusion(&dense, &sparse, 60, 10);

        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results not sorted: {} < {} at positions {} and {}",
                results[i - 1].score,
                results[i].score,
                i - 1,
                i
            );
        }
    }

    #[test]
    fn test_rrf_rank_tracking() {
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(2, 5.0), (3, 4.0)];

        let results = rrf_fusion(&dense, &sparse, 60, 10);

        // Find result for ID 1 (only in dense)
        let r1 = results.iter().find(|r| r.id == 1).unwrap();
        assert_eq!(r1.dense_rank, Some(1));
        assert_eq!(r1.sparse_rank, None);

        // Find result for ID 2 (in both)
        let r2 = results.iter().find(|r| r.id == 2).unwrap();
        assert_eq!(r2.dense_rank, Some(2));
        assert_eq!(r2.sparse_rank, Some(1));

        // Find result for ID 3 (only in sparse)
        let r3 = results.iter().find(|r| r.id == 3).unwrap();
        assert_eq!(r3.dense_rank, None);
        assert_eq!(r3.sparse_rank, Some(2));
    }

    #[test]
    fn test_rrf_large_lists() {
        // Test with larger lists
        let dense: Vec<(u64, f32)> = (0..1000)
            .map(|i| (i as u64, 1.0 - (i as f32 / 1000.0)))
            .collect();
        let sparse: Vec<(u64, f32)> = (500..1500)
            .map(|i| (i as u64, 1.0 - ((i - 500) as f32 / 1000.0)))
            .collect();

        let results = rrf_fusion(&dense, &sparse, 60, 100);

        assert_eq!(results.len(), 100);

        // IDs 500-999 appear in both lists, should generally rank higher
        let overlap_count = results.iter()
            .filter(|r| r.dense_rank.is_some() && r.sparse_rank.is_some())
            .count();

        // Most top results should be from overlap
        assert!(overlap_count > 50, "Expected most results from overlap, got {}", overlap_count);
    }

    // ============= FusionResult Tests =============

    #[test]
    fn test_fusion_result_new() {
        let result = FusionResult::new(42, 0.5);
        assert_eq!(result.id, 42);
        assert_eq!(result.score, 0.5);
        assert_eq!(result.dense_rank, None);
        assert_eq!(result.sparse_rank, None);
    }

    #[test]
    fn test_fusion_result_with_ranks() {
        let result = FusionResult::with_ranks(42, 0.5, Some(1), Some(2));
        assert_eq!(result.id, 42);
        assert_eq!(result.score, 0.5);
        assert_eq!(result.dense_rank, Some(1));
        assert_eq!(result.sparse_rank, Some(2));
    }

    // ============= FusionMethod Tests =============

    #[test]
    fn test_fusion_method_default() {
        let method = FusionMethod::default();
        match method {
            FusionMethod::Rrf { k } => assert_eq!(k, 60),
            _ => panic!("Expected RRF"),
        }
    }

    #[test]
    fn test_fusion_method_rrf() {
        let method = FusionMethod::rrf();
        match method {
            FusionMethod::Rrf { k } => assert_eq!(k, RRF_DEFAULT_K),
            _ => panic!("Expected RRF"),
        }
    }

    #[test]
    fn test_fusion_method_rrf_with_k() {
        let method = FusionMethod::rrf_with_k(100);
        match method {
            FusionMethod::Rrf { k } => assert_eq!(k, 100),
            _ => panic!("Expected RRF"),
        }
    }

    #[test]
    fn test_fusion_method_linear() {
        let method = FusionMethod::linear(0.7);
        match method {
            FusionMethod::Linear { alpha } => assert_eq!(alpha, 0.7),
            _ => panic!("Expected Linear"),
        }
    }

    #[test]
    fn test_fusion_method_linear_balanced() {
        let method = FusionMethod::linear_balanced();
        match method {
            FusionMethod::Linear { alpha } => assert_eq!(alpha, 0.5),
            _ => panic!("Expected Linear"),
        }
    }

    #[test]
    #[should_panic(expected = "Alpha must be in range")]
    fn test_fusion_method_linear_invalid_high() {
        FusionMethod::linear(1.5);
    }

    #[test]
    #[should_panic(expected = "Alpha must be in range")]
    fn test_fusion_method_linear_invalid_low() {
        FusionMethod::linear(-0.1);
    }
}
```

**Acceptance Criteria:**
- [ ] `test_rrf_identical_lists` - Same order gives expected scores
- [ ] `test_rrf_disjoint_lists` - No overlap handled
- [ ] `test_rrf_partial_overlap` - Mixed case with correct scores
- [ ] `test_rrf_k_parameter_effect` - Different k values work
- [ ] `test_rrf_empty_dense` - Dense empty handled
- [ ] `test_rrf_empty_sparse` - Sparse empty handled
- [ ] `test_rrf_both_empty` - Both empty handled
- [ ] `test_rrf_top_n_zero` - Zero returns empty
- [ ] `test_rrf_top_n_truncation` - Truncation works
- [ ] `test_rrf_score_ordering` - Results sorted
- [ ] `test_rrf_rank_tracking` - Ranks recorded correctly
- [ ] `test_rrf_large_lists` - Performance with 1k items
- [ ] All FusionResult tests pass
- [ ] All FusionMethod tests pass
- [ ] All tests pass: `cargo test --features sparse hybrid`

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

## Day 2 Checklist

- [ ] W39.2.1: `src/hybrid/mod.rs` created with module structure
- [ ] W39.2.2: `FusionResult` and `FusionMethod` types implemented
- [ ] W39.2.3: `rrf_fusion()` function implemented with correct algorithm
- [ ] W39.2.4: All 20+ unit tests passing
- [ ] `hybrid` module exported from `lib.rs`
- [ ] `cargo check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test hybrid` passes
- [ ] `cargo doc` generates correct documentation

---

## Day 2 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| RRF formula correct | `test_rrf_identical_lists`, `test_rrf_partial_overlap` |
| Handles edge cases | Empty list tests |
| Results sorted | `test_rrf_score_ordering` |
| Rank tracking | `test_rrf_rank_tracking` |
| FusionMethod API | Factory method tests |
| Performance | `test_rrf_large_lists` (1k items) |
| Clippy clean | `cargo clippy -- -D warnings` |

---

## Day 2 Handoff

After completing Day 2:

**Artifacts Generated:**
- `src/hybrid/mod.rs` - Module exports
- `src/hybrid/fusion.rs` - RRF fusion implementation
- 20+ unit tests for fusion

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 3 - `HybridSearcher` and `hybridSearch()` API

---

## Notes for Implementation

### RRF Score Interpretation

- RRF scores are NOT similarity scores
- They represent "rank-based agreement" between lists
- Higher score = document is highly ranked in both/multiple lists
- Maximum score for 2 lists: `2 / (k + 1)` (both rank 1)
- Typical range: 0.01 - 0.03 for k=60

### Why k=60?

The original paper found k=60 to be robust across datasets:
- k too low (e.g., 1): First-ranked items dominate excessively
- k too high (e.g., 1000): Ranks have minimal differentiation
- k=60: Good balance for most retrieval scenarios

### Memory Efficiency

The HashMap approach is O(u) where u = unique IDs. For typical
retrieval (dense_k=100, sparse_k=100), this is at most 200 entries.
No optimization needed for v0.9.0.

---

*Agent: PLANNER*
*Status: [PROPOSED]*
*Date: 2026-01-21*

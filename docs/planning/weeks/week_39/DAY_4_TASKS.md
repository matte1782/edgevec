# Week 39 Day 4: Linear Combination Fusion

**Date:** 2026-01-29
**Focus:** Implement alpha-weighted linear fusion with score normalization
**Estimated Duration:** 2 hours
**Phase:** RFC-007 Implementation Phase 3 (Hybrid Search)
**Dependencies:** Day 2 (Fusion types), Day 3 (HybridSearcher framework)

---

## Context

Day 4 implements the linear combination fusion mode that provides fine-grained
control over the balance between dense (semantic) and sparse (keyword) relevance.

**Linear Fusion Formula:**
```
final_score = alpha * normalized_dense + (1 - alpha) * normalized_sparse
```

Where scores are min-max normalized to [0, 1] before combination.

**Use Cases:**
- `alpha = 0.7`: Prioritize semantic similarity (70% dense, 30% sparse)
- `alpha = 0.3`: Prioritize keyword matching (30% dense, 70% sparse)
- `alpha = 0.5`: Equal weight (balanced hybrid)

---

## Tasks

### W39.4.1: Implement Score Normalization Helpers

**Objective:** Create helper functions for min-max normalization.

**File:** `src/hybrid/fusion.rs` (additions)

```rust
/// Min-max normalize scores to [0, 1] range.
///
/// # Formula
///
/// `normalized = (score - min) / (max - min)`
///
/// # Edge Cases
///
/// - Empty input: returns empty Vec
/// - Single element: returns 1.0 (max normalized)
/// - All same scores: returns 1.0 for all (treated as max)
///
/// # Assumptions [HOSTILE_REVIEW: m4 Resolution]
///
/// Input scores are expected to be non-negative (>= 0.0). This is true for:
/// - Dense: cosine similarity in [0, 1] or [-1, 1] (we use distance which is >= 0)
/// - Sparse: dot product of non-negative BM25/TF-IDF scores (>= 0)
///
/// If negative scores are possible, add debug_assert!(scores.iter().all(|(_, s)| *s >= 0.0))
/// or handle by shifting scores before normalization.
///
/// # Arguments
///
/// * `results` - Slice of (id, score) tuples
///
/// # Returns
///
/// Vec of (id, normalized_score) tuples with scores in [0, 1].
fn normalize_scores(results: &[(u64, f32)]) -> Vec<(u64, f32)> {
    if results.is_empty() {
        return Vec::new();
    }

    if results.len() == 1 {
        // Single result gets max normalized score
        return vec![(results[0].0, 1.0)];
    }

    // Find min and max scores
    let mut min_score = f32::MAX;
    let mut max_score = f32::MIN;

    for (_, score) in results {
        if *score < min_score {
            min_score = *score;
        }
        if *score > max_score {
            max_score = *score;
        }
    }

    let range = max_score - min_score;

    // Handle case where all scores are the same
    if range == 0.0 || range.abs() < f32::EPSILON {
        return results.iter().map(|(id, _)| (*id, 1.0)).collect();
    }

    // Normalize each score
    results
        .iter()
        .map(|(id, score)| {
            let normalized = (score - min_score) / range;
            (*id, normalized)
        })
        .collect()
}

/// Normalize scores with explicit min/max (for testing).
#[cfg(test)]
fn normalize_scores_with_range(results: &[(u64, f32)], min: f32, max: f32) -> Vec<(u64, f32)> {
    let range = max - min;
    if range == 0.0 || range.abs() < f32::EPSILON {
        return results.iter().map(|(id, _)| (*id, 1.0)).collect();
    }

    results
        .iter()
        .map(|(id, score)| {
            let normalized = (score - min) / range;
            // Clamp to [0, 1] in case score is outside [min, max]
            (*id, normalized.clamp(0.0, 1.0))
        })
        .collect()
}
```

**Acceptance Criteria:**
- [ ] `normalize_scores()` maps to [0, 1] range
- [ ] Empty input returns empty Vec
- [ ] Single element returns 1.0
- [ ] All same scores return 1.0 for all
- [ ] Min score maps to 0.0, max to 1.0
- [ ] Middle scores map proportionally

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W39.4.2: Implement `linear_fusion()` Function

**Objective:** Implement linear combination with score normalization.

**File:** `src/hybrid/fusion.rs` (continued)

```rust
/// Linear combination fusion.
///
/// Combines normalized scores using:
/// `final_score = alpha * dense_score + (1 - alpha) * sparse_score`
///
/// # Score Normalization
///
/// Before combination, scores are min-max normalized to [0, 1]:
/// - Min score in each list maps to 0.0
/// - Max score in each list maps to 1.0
/// - Missing documents get 0.0 (treated as min relevance in that system)
///
/// # Arguments
///
/// * `dense_results` - Results from dense search with similarity scores,
///   ordered by descending score.
/// * `sparse_results` - Results from sparse search with dot product scores,
///   ordered by descending score.
/// * `alpha` - Weight for dense scores (0.0 to 1.0).
///   - alpha=0.0: sparse only
///   - alpha=1.0: dense only
///   - alpha=0.5: balanced
/// * `top_n` - Number of results to return.
///
/// # Returns
///
/// Vec of `FusionResult` containing fused results sorted by descending
/// combined score. Includes rank information from both lists.
///
/// # Example
///
/// ```rust
/// use edgevec::hybrid::linear_fusion;
///
/// // Dense results (similarity scores)
/// let dense = vec![(1, 0.95), (2, 0.80), (3, 0.75)];
///
/// // Sparse results (dot product scores)
/// let sparse = vec![(2, 5.5), (4, 4.2), (1, 3.8)];
///
/// // 70% weight on dense, 30% on sparse
/// let results = linear_fusion(&dense, &sparse, 0.7, 10);
///
/// // ID 1: dense normalized = 1.0 (rank 1), sparse normalized ~0.0 (rank 3 of 3)
/// // ID 2: dense normalized ~0.5 (rank 2), sparse normalized = 1.0 (rank 1)
/// // Combined scores determine final ordering
/// ```
#[must_use]
pub fn linear_fusion(
    dense_results: &[(u64, f32)],
    sparse_results: &[(u64, f32)],
    alpha: f32,
    top_n: usize,
) -> Vec<FusionResult> {
    if top_n == 0 {
        return Vec::new();
    }

    // Validate alpha
    let alpha = alpha.clamp(0.0, 1.0);

    // Normalize scores
    let dense_normalized = normalize_scores(dense_results);
    let sparse_normalized = normalize_scores(sparse_results);

    // Build lookup maps for normalized scores
    let dense_map: HashMap<u64, f32> = dense_normalized.iter().copied().collect();
    let sparse_map: HashMap<u64, f32> = sparse_normalized.iter().copied().collect();

    // Build rank maps
    let dense_ranks: HashMap<u64, usize> = dense_results
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i + 1))
        .collect();
    let sparse_ranks: HashMap<u64, usize> = sparse_results
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i + 1))
        .collect();

    // Collect all unique IDs
    let mut all_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for (id, _) in dense_results {
        all_ids.insert(*id);
    }
    for (id, _) in sparse_results {
        all_ids.insert(*id);
    }

    // Calculate combined scores
    let mut results: Vec<FusionResult> = all_ids
        .into_iter()
        .map(|id| {
            // Get normalized scores (0.0 if not present)
            let dense_score = dense_map.get(&id).copied().unwrap_or(0.0);
            let sparse_score = sparse_map.get(&id).copied().unwrap_or(0.0);

            // Linear combination
            let combined = alpha * dense_score + (1.0 - alpha) * sparse_score;

            FusionResult::with_ranks(
                id,
                combined,
                dense_ranks.get(&id).copied(),
                sparse_ranks.get(&id).copied(),
            )
        })
        .collect();

    // Sort by descending combined score
    results.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Truncate to top_n
    results.truncate(top_n);

    results
}
```

**Acceptance Criteria:**
- [ ] Correct formula: `alpha * dense + (1 - alpha) * sparse`
- [ ] Scores normalized before combination
- [ ] Missing documents get 0.0 (penalized)
- [ ] alpha=0.0 gives sparse-only results
- [ ] alpha=1.0 gives dense-only results
- [ ] alpha=0.5 gives balanced combination
- [ ] Results sorted by descending combined score
- [ ] Handles empty input lists
- [ ] Doc comments with examples

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W39.4.3: Unit Tests for Linear Fusion

**Objective:** Comprehensive test coverage for linear fusion.

**File:** `src/hybrid/fusion.rs` (tests module additions)

```rust
// Add to existing tests module

    // ============= Normalization Tests =============

    #[test]
    fn test_normalize_empty() {
        let results: Vec<(u64, f32)> = vec![];
        let normalized = normalize_scores(&results);
        assert!(normalized.is_empty());
    }

    #[test]
    fn test_normalize_single() {
        let results = vec![(1, 0.5)];
        let normalized = normalize_scores(&results);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].1, 1.0);
    }

    #[test]
    fn test_normalize_all_same() {
        let results = vec![(1, 0.5), (2, 0.5), (3, 0.5)];
        let normalized = normalize_scores(&results);
        assert_eq!(normalized.len(), 3);
        for (_, score) in &normalized {
            assert_eq!(*score, 1.0);
        }
    }

    #[test]
    fn test_normalize_range() {
        let results = vec![(1, 0.0), (2, 0.5), (3, 1.0)];
        let normalized = normalize_scores(&results);

        // Find each ID
        let n1 = normalized.iter().find(|(id, _)| *id == 1).unwrap().1;
        let n2 = normalized.iter().find(|(id, _)| *id == 2).unwrap().1;
        let n3 = normalized.iter().find(|(id, _)| *id == 3).unwrap().1;

        assert!((n1 - 0.0).abs() < 1e-6); // min -> 0.0
        assert!((n2 - 0.5).abs() < 1e-6); // middle -> 0.5
        assert!((n3 - 1.0).abs() < 1e-6); // max -> 1.0
    }

    #[test]
    fn test_normalize_large_range() {
        let results = vec![(1, 0.0), (2, 100.0)];
        let normalized = normalize_scores(&results);

        let n1 = normalized.iter().find(|(id, _)| *id == 1).unwrap().1;
        let n2 = normalized.iter().find(|(id, _)| *id == 2).unwrap().1;

        assert!((n1 - 0.0).abs() < 1e-6);
        assert!((n2 - 1.0).abs() < 1e-6);
    }

    // ============= Linear Fusion Tests =============

    #[test]
    fn test_linear_alpha_zero() {
        // alpha=0.0 means sparse only
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(3, 5.0), (4, 4.0)];

        let results = linear_fusion(&dense, &sparse, 0.0, 10);

        // Sparse IDs should rank higher
        assert_eq!(results[0].id, 3); // Best sparse
        assert_eq!(results[1].id, 4); // Second sparse
    }

    #[test]
    fn test_linear_alpha_one() {
        // alpha=1.0 means dense only
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(3, 5.0), (4, 4.0)];

        let results = linear_fusion(&dense, &sparse, 1.0, 10);

        // Dense IDs should rank higher
        assert_eq!(results[0].id, 1); // Best dense
        assert_eq!(results[1].id, 2); // Second dense
    }

    #[test]
    fn test_linear_alpha_half() {
        // alpha=0.5 means balanced
        // IDs in both lists should rank highest
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(2, 5.0), (3, 4.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // ID 2 appears in both, should have combined advantage
        // ID 2 dense: normalized ~0.0 (it's rank 2 of 2, score 0.8)
        // Wait, need to reconsider - normalization is within each list

        // Dense: 1 -> 1.0 (max), 2 -> 0.0 (min)
        // Sparse: 2 -> 1.0 (max), 3 -> 0.0 (min)

        // ID 1: 0.5*1.0 + 0.5*0.0 = 0.5 (dense only)
        // ID 2: 0.5*0.0 + 0.5*1.0 = 0.5 (sparse only)
        // ID 3: 0.5*0.0 + 0.5*0.0 = 0.0 (sparse low)

        // IDs 1 and 2 should tie at 0.5
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_linear_both_lists_overlap() {
        // Same ID in both lists gets combined score
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(1, 5.0), (2, 4.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // Both IDs in both lists
        // ID 1: dense=1.0, sparse=1.0 -> 0.5*1.0 + 0.5*1.0 = 1.0
        // ID 2: dense=0.0, sparse=0.0 -> 0.5*0.0 + 0.5*0.0 = 0.0
        assert_eq!(results[0].id, 1);
        assert!((results[0].score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_linear_disjoint_lists() {
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(3, 5.0), (4, 4.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        assert_eq!(results.len(), 4);

        // Check all IDs present
        let ids: Vec<u64> = results.iter().map(|r| r.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
        assert!(ids.contains(&4));
    }

    #[test]
    fn test_linear_empty_dense() {
        let dense: Vec<(u64, f32)> = vec![];
        let sparse = vec![(1, 5.0), (2, 4.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // With alpha=0.5 and no dense, max sparse score = 0.5*1.0 = 0.5
        assert_eq!(results.len(), 2);
        assert!(results[0].score <= 0.5 + 1e-6);
    }

    #[test]
    fn test_linear_empty_sparse() {
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse: Vec<(u64, f32)> = vec![];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // With alpha=0.5 and no sparse, max dense score = 0.5*1.0 = 0.5
        assert_eq!(results.len(), 2);
        assert!(results[0].score <= 0.5 + 1e-6);
    }

    #[test]
    fn test_linear_both_empty() {
        let dense: Vec<(u64, f32)> = vec![];
        let sparse: Vec<(u64, f32)> = vec![];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_linear_top_n_zero() {
        let dense = vec![(1, 0.9)];
        let sparse = vec![(1, 5.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_linear_alpha_boundary_low() {
        // alpha slightly above 0
        let dense = vec![(1, 0.9)];
        let sparse = vec![(2, 5.0)];

        let results = linear_fusion(&dense, &sparse, 0.01, 10);

        // Sparse should dominate
        assert_eq!(results[0].id, 2);
    }

    #[test]
    fn test_linear_alpha_boundary_high() {
        // alpha slightly below 1
        let dense = vec![(1, 0.9)];
        let sparse = vec![(2, 5.0)];

        let results = linear_fusion(&dense, &sparse, 0.99, 10);

        // Dense should dominate
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_linear_preserves_ranks() {
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(2, 5.0), (3, 4.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // Check rank tracking
        let r1 = results.iter().find(|r| r.id == 1).unwrap();
        assert_eq!(r1.dense_rank, Some(1));
        assert_eq!(r1.sparse_rank, None);

        let r2 = results.iter().find(|r| r.id == 2).unwrap();
        assert_eq!(r2.dense_rank, Some(2));
        assert_eq!(r2.sparse_rank, Some(1));

        let r3 = results.iter().find(|r| r.id == 3).unwrap();
        assert_eq!(r3.dense_rank, None);
        assert_eq!(r3.sparse_rank, Some(2));
    }

    #[test]
    fn test_linear_score_ordering() {
        let dense = vec![(1, 0.9), (2, 0.8), (3, 0.7)];
        let sparse = vec![(4, 5.0), (5, 4.0), (6, 3.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // Check descending order
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results not sorted: {} < {}",
                results[i - 1].score,
                results[i].score
            );
        }
    }

    #[test]
    fn test_linear_alpha_clamping() {
        // Alpha out of range should be clamped
        let dense = vec![(1, 0.9)];
        let sparse = vec![(2, 5.0)];

        // Should not panic, should clamp to 1.0
        let results = linear_fusion(&dense, &sparse, 1.5, 10);
        assert!(!results.is_empty());

        // Should not panic, should clamp to 0.0
        let results = linear_fusion(&dense, &sparse, -0.5, 10);
        assert!(!results.is_empty());
    }
```

**Acceptance Criteria:**
- [ ] `test_normalize_*` - All normalization tests pass
- [ ] `test_linear_alpha_zero` - Pure sparse works
- [ ] `test_linear_alpha_one` - Pure dense works
- [ ] `test_linear_alpha_half` - Balanced works
- [ ] `test_linear_both_lists_overlap` - Combined scoring
- [ ] `test_linear_disjoint_lists` - No overlap handled
- [ ] `test_linear_empty_*` - Empty list handling
- [ ] `test_linear_preserves_ranks` - Rank tracking
- [ ] `test_linear_score_ordering` - Results sorted
- [ ] `test_linear_alpha_clamping` - Out-of-range alpha clamped
- [ ] All tests pass: `cargo test hybrid::fusion`

**Estimated Duration:** 45 minutes

**Agent:** TEST_ENGINEER

---

## Day 4 Checklist

- [ ] W39.4.1: `normalize_scores()` helper implemented
- [ ] W39.4.2: `linear_fusion()` function implemented
- [ ] W39.4.3: All 15+ unit tests passing
- [ ] Linear fusion integrated with `HybridSearcher` (from Day 3)
- [ ] `cargo check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test hybrid::fusion` passes

---

## Day 4 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Normalization correct | `test_normalize_range` |
| alpha=0.0 works | `test_linear_alpha_zero` |
| alpha=1.0 works | `test_linear_alpha_one` |
| alpha=0.5 balanced | `test_linear_alpha_half` |
| Missing docs penalized | `test_linear_disjoint_lists` |
| Results sorted | `test_linear_score_ordering` |
| Clippy clean | `cargo clippy -- -D warnings` |

---

## Day 4 Handoff

After completing Day 4:

**Artifacts Generated:**
- Updated `src/hybrid/fusion.rs` with `linear_fusion()` and `normalize_scores()`
- 15+ additional unit tests

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 5 - WASM Bindings + TypeScript Types

---

## Notes for Implementation

### Why Min-Max Normalization?

Dense (similarity) and sparse (dot product) scores have different scales:
- Dense: typically 0.0 to 1.0 (cosine similarity)
- Sparse: can be 0 to 100+ (dot product)

Min-max normalization puts both on [0, 1] scale for fair combination.

### Missing Document Penalty

When a document appears in only one list, it gets 0.0 for the other:
- Dense only: `alpha * 1.0 + (1 - alpha) * 0.0 = alpha`
- Sparse only: `alpha * 0.0 + (1 - alpha) * 1.0 = 1 - alpha`

With alpha=0.5, a document in both lists (scoring 1.0 in both) gets
score 1.0, while a document in only one list gets at most 0.5.
This naturally prefers documents with evidence from both systems.

### Alpha Tuning Guidelines

| Alpha | Use Case |
|:------|:---------|
| 0.7 | Strong semantic understanding, weak keywords |
| 0.5 | Balanced (start here for tuning) |
| 0.3 | Important keywords, weaker semantics |
| 0.9 | Nearly pure semantic (keyword as tiebreaker) |
| 0.1 | Nearly pure keyword (semantic as tiebreaker) |

---

*Agent: PLANNER*
*Status: [PROPOSED]*
*Date: 2026-01-21*

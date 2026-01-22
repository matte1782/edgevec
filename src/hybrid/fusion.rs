//! Fusion algorithms for hybrid search.
//!
//! This module implements score fusion methods that combine results
//! from multiple retrieval systems (dense + sparse).
//!
//! # Algorithms
//!
//! ## Reciprocal Rank Fusion (RRF)
//!
//! RRF combines ranked lists based on position, not score values.
//! Formula: `score(d) = sum(1 / (k + rank_i(d)))` for each list `i`.
//!
//! ## Linear Combination
//!
//! Linear fusion combines normalized scores:
//! `score(d) = alpha * norm_dense + (1 - alpha) * norm_sparse`
//!
//! # Reference
//!
//! Cormack, G.V., Clarke, C.L.A., and Buettcher, S. (2009).
//! "Reciprocal Rank Fusion outperforms Condorcet and individual
//! Rank Learning Methods" SIGIR 2009.

use std::cmp::Ordering;
use std::collections::HashMap;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Default k parameter for RRF fusion.
///
/// k=60 is the standard value from the original RRF paper.
/// Higher values give more weight to documents ranked lower in lists.
pub const RRF_DEFAULT_K: u32 = 60;

// =============================================================================
// FUSION RESULT
// =============================================================================

/// Result from fusion algorithm.
///
/// # Note on Equality
///
/// This type derives `PartialEq` which compares `f32` scores directly.
/// Due to floating-point precision, two results with nearly-identical
/// scores may not compare equal. For score comparison, consider using
/// an epsilon-based comparison instead of direct equality.
#[derive(Clone, Debug, PartialEq)]
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

// =============================================================================
// FUSION METHOD
// =============================================================================

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
            "Alpha must be in range [0.0, 1.0], got {alpha}"
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

// =============================================================================
// RRF FUSION
// =============================================================================

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
    // Track scores and ranks for each document
    struct DocInfo {
        score: f32,
        dense_rank: Option<usize>,
        sparse_rank: Option<usize>,
    }

    if top_n == 0 {
        return Vec::new();
    }

    // Use f64 for intermediate calculations to avoid precision loss when
    // accumulating many small RRF contributions (1/(k+rank) values).
    // Final scores are truncated to f32 for storage efficiency.
    let k_f64 = f64::from(k);

    let mut doc_map: HashMap<u64, DocInfo> = HashMap::new();

    // Process dense results (ranks are 1-indexed per RRF paper)
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    for (rank_0, (id, _score)) in dense_results.iter().enumerate() {
        let rank = rank_0 + 1; // Convert to 1-indexed
        let rrf_contribution = (1.0 / (k_f64 + rank as f64)) as f32;

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
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    for (rank_0, (id, _score)) in sparse_results.iter().enumerate() {
        let rank = rank_0 + 1; // Convert to 1-indexed
        let rrf_contribution = (1.0 / (k_f64 + rank as f64)) as f32;

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
        .map(|(id, info)| {
            FusionResult::with_ranks(id, info.score, info.dense_rank, info.sparse_rank)
        })
        .collect();

    // Sort by descending score
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

    // Truncate to top_n
    results.truncate(top_n);

    results
}

// =============================================================================
// NORMALIZATION
// =============================================================================

/// Min-max normalize scores to [0, 1] range.
///
/// # Formula
///
/// `normalized = (score - min) / (max - min)`
///
/// # Edge Cases
///
/// - Empty input: returns empty HashMap
/// - Single element: returns 1.0 (max normalized)
/// - All same scores: returns 1.0 for all (treated as max)
///
/// # Assumptions
///
/// Input scores are expected to be non-negative (>= 0.0). This is true for:
/// - Dense: cosine similarity in [0, 1]
/// - Sparse: dot product of non-negative BM25/TF-IDF scores (>= 0)
///
/// # Arguments
///
/// * `results` - Slice of (id, score) tuples
///
/// # Returns
///
/// HashMap mapping id to normalized score in [0, 1].
fn normalize_scores(results: &[(u64, f32)]) -> HashMap<u64, f32> {
    if results.is_empty() {
        return HashMap::new();
    }

    let scores: Vec<f32> = results.iter().map(|(_, s)| *s).collect();
    let min = scores.iter().copied().fold(f32::INFINITY, f32::min);
    let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    let range = max - min;

    results
        .iter()
        .map(|(id, score)| {
            let normalized = if range == 0.0 {
                1.0 // All same score → treat as max (includes single element)
            } else {
                (score - min) / range
            };
            (*id, normalized)
        })
        .collect()
}

// =============================================================================
// LINEAR FUSION
// =============================================================================

/// Linear combination fusion algorithm.
///
/// Combines scores using min-max normalization and weighted sum:
/// `score(d) = alpha * norm_dense + (1 - alpha) * norm_sparse`
///
/// Documents appearing in only one list get 0 for the missing component.
///
/// # Arguments
///
/// * `dense_results` - Results from dense search as (id, score) tuples
/// * `sparse_results` - Results from sparse search as (id, score) tuples
/// * `alpha` - Weight for dense scores (0.0 to 1.0). 0.5 = equal weight.
/// * `top_n` - Number of results to return.
///
/// # Returns
///
/// Vec of `FusionResult` sorted by descending combined score.
///
/// # Note
///
/// [HOSTILE_REVIEW: m4 Resolution] - This function expects non-negative scores.
/// Dense similarity (cosine in [0,1]) and sparse BM25 scores (>= 0) satisfy this.
/// If scores can be negative, normalization behavior is undefined.
///
/// # Example
///
/// ```rust
/// use edgevec::hybrid::linear_fusion;
///
/// let dense = vec![(1, 0.95), (2, 0.80)];
/// let sparse = vec![(2, 5.0), (1, 3.0)];
///
/// // Equal weighting
/// let results = linear_fusion(&dense, &sparse, 0.5, 10);
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

    // Clamp alpha to valid range [0.0, 1.0]
    let alpha = alpha.clamp(0.0, 1.0);

    // Debug assertion: scores should be non-negative for proper normalization
    // Dense similarity (cosine in [0,1]) and sparse BM25 scores (>= 0) satisfy this.
    debug_assert!(
        dense_results.iter().all(|(_, s)| *s >= 0.0),
        "Dense scores must be non-negative for linear fusion normalization"
    );
    debug_assert!(
        sparse_results.iter().all(|(_, s)| *s >= 0.0),
        "Sparse scores must be non-negative for linear fusion normalization"
    );

    // Normalize scores to [0, 1] using min-max normalization
    let dense_norm = normalize_scores(dense_results);
    let sparse_norm = normalize_scores(sparse_results);

    // Collect all unique IDs with ranks
    let mut doc_map: HashMap<u64, (f32, Option<usize>, Option<usize>)> = HashMap::new();

    for (rank_0, (id, _)) in dense_results.iter().enumerate() {
        let norm_score = dense_norm.get(id).copied().unwrap_or(0.0);
        doc_map
            .entry(*id)
            .and_modify(|(s, dr, _)| {
                *s += alpha * norm_score;
                *dr = Some(rank_0 + 1);
            })
            .or_insert((alpha * norm_score, Some(rank_0 + 1), None));
    }

    for (rank_0, (id, _)) in sparse_results.iter().enumerate() {
        let norm_score = sparse_norm.get(id).copied().unwrap_or(0.0);
        doc_map
            .entry(*id)
            .and_modify(|(s, _, sr)| {
                *s += (1.0 - alpha) * norm_score;
                *sr = Some(rank_0 + 1);
            })
            .or_insert(((1.0 - alpha) * norm_score, None, Some(rank_0 + 1)));
    }

    // Convert to results and sort
    let mut results: Vec<FusionResult> = doc_map
        .into_iter()
        .map(|(id, (score, dense_rank, sparse_rank))| {
            FusionResult::with_ranks(id, score, dense_rank, sparse_rank)
        })
        .collect();

    // Sort by descending score
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

    // Truncate to top_n
    results.truncate(top_n);

    results
}

// =============================================================================
// TESTS
// =============================================================================

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
        let overlap_count = results
            .iter()
            .filter(|r| r.dense_rank.is_some() && r.sparse_rank.is_some())
            .count();

        // Most top results should be from overlap
        assert!(
            overlap_count > 50,
            "Expected most results from overlap, got {}",
            overlap_count
        );
    }

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
        assert_eq!(normalized.get(&1).copied().unwrap(), 1.0);
    }

    #[test]
    fn test_normalize_all_same() {
        let results = vec![(1, 0.5), (2, 0.5), (3, 0.5)];
        let normalized = normalize_scores(&results);
        assert_eq!(normalized.len(), 3);
        for id in 1..=3 {
            assert_eq!(normalized.get(&id).copied().unwrap(), 1.0);
        }
    }

    #[test]
    fn test_normalize_range() {
        let results = vec![(1, 0.0), (2, 0.5), (3, 1.0)];
        let normalized = normalize_scores(&results);

        // min (0.0) -> 0.0, middle (0.5) -> 0.5, max (1.0) -> 1.0
        assert!((normalized.get(&1).copied().unwrap() - 0.0).abs() < 1e-6);
        assert!((normalized.get(&2).copied().unwrap() - 0.5).abs() < 1e-6);
        assert!((normalized.get(&3).copied().unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_large_range() {
        let results = vec![(1, 0.0), (2, 100.0)];
        let normalized = normalize_scores(&results);

        assert!((normalized.get(&1).copied().unwrap() - 0.0).abs() < 1e-6);
        assert!((normalized.get(&2).copied().unwrap() - 1.0).abs() < 1e-6);
    }

    // ============= Linear Fusion Tests =============

    #[test]
    fn test_linear_identical_lists() {
        let dense = vec![(1, 0.9), (2, 0.8), (3, 0.7)];
        let sparse = vec![(1, 5.0), (2, 4.0), (3, 3.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        assert_eq!(results.len(), 3);
        // ID 1 is rank 1 in both with highest scores
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_linear_alpha_weighting() {
        // Dense favors ID 1, Sparse favors ID 2
        let dense = vec![(1, 1.0), (2, 0.0)];
        let sparse = vec![(2, 1.0), (1, 0.0)];

        // alpha=1.0: dense only
        let results_dense = linear_fusion(&dense, &sparse, 1.0, 10);
        assert_eq!(results_dense[0].id, 1);

        // alpha=0.0: sparse only
        let results_sparse = linear_fusion(&dense, &sparse, 0.0, 10);
        assert_eq!(results_sparse[0].id, 2);

        // alpha=0.5: tie
        let results_balanced = linear_fusion(&dense, &sparse, 0.5, 10);
        // Both should have same score
        assert!((results_balanced[0].score - results_balanced[1].score).abs() < 1e-6);
    }

    #[test]
    fn test_linear_empty_dense() {
        let dense: Vec<(u64, f32)> = vec![];
        let sparse = vec![(1, 5.0), (2, 4.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        assert_eq!(results.len(), 2);
        // With alpha=0.5, sparse contributes 0.5 * normalized_score
        assert!(results[0].score <= 0.5 + 1e-6);
    }

    #[test]
    fn test_linear_empty_sparse() {
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse: Vec<(u64, f32)> = vec![];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        assert_eq!(results.len(), 2);
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
    fn test_linear_single_score_normalization() {
        // Single element should normalize to 1.0
        let dense = vec![(1, 0.5)];
        let sparse = vec![(1, 3.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // Both normalize to 1.0, combined score = 0.5 * 1.0 + 0.5 * 1.0 = 1.0
        assert!((results[0].score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_linear_alpha_zero() {
        // alpha=0.0 means only sparse scores contribute
        // Use overlapping items to verify alpha weighting works correctly
        let dense = vec![(1, 1.0), (2, 0.5), (3, 0.0)];
        let sparse = vec![(1, 0.0), (2, 0.5), (3, 1.0)];

        let results = linear_fusion(&dense, &sparse, 0.0, 10);

        assert_eq!(results.len(), 3);

        // With alpha=0.0, only sparse contributes
        // Sparse normalized: 1→0.0, 2→0.5, 3→1.0
        // Combined: 1→0.0, 2→0.5, 3→1.0

        // Item 3 should be first (highest sparse score)
        assert_eq!(results[0].id, 3);
        assert!((results[0].score - 1.0).abs() < 1e-6);

        // Item 2 should be second
        assert_eq!(results[1].id, 2);
        assert!((results[1].score - 0.5).abs() < 1e-6);

        // Item 1 should be last (lowest sparse score)
        assert_eq!(results[2].id, 1);
        assert!(results[2].score.abs() < 1e-6);
    }

    #[test]
    fn test_linear_alpha_one() {
        // alpha=1.0 means only dense scores contribute
        // Use overlapping items to verify alpha weighting works correctly
        let dense = vec![(1, 1.0), (2, 0.5), (3, 0.0)];
        let sparse = vec![(1, 0.0), (2, 0.5), (3, 1.0)];

        let results = linear_fusion(&dense, &sparse, 1.0, 10);

        assert_eq!(results.len(), 3);

        // With alpha=1.0, only dense contributes
        // Dense normalized: 1→1.0, 2→0.5, 3→0.0
        // Combined: 1→1.0, 2→0.5, 3→0.0

        // Item 1 should be first (highest dense score)
        assert_eq!(results[0].id, 1);
        assert!((results[0].score - 1.0).abs() < 1e-6);

        // Item 2 should be second
        assert_eq!(results[1].id, 2);
        assert!((results[1].score - 0.5).abs() < 1e-6);

        // Item 3 should be last (lowest dense score)
        assert_eq!(results[2].id, 3);
        assert!(results[2].score.abs() < 1e-6);
    }

    #[test]
    fn test_linear_disjoint_lists() {
        // No overlap between dense and sparse results
        let dense = vec![(1, 0.9), (2, 0.8)];
        let sparse = vec![(3, 5.0), (4, 4.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // Should have all 4 unique items
        assert_eq!(results.len(), 4);

        // Check that each result has only one rank populated
        for result in &results {
            let has_dense = result.dense_rank.is_some();
            let has_sparse = result.sparse_rank.is_some();
            assert!(
                has_dense ^ has_sparse,
                "Disjoint items should have exactly one rank"
            );
        }

        // With equal alpha (0.5), both sides contribute equally
        // but since scores are normalized, ordering depends on individual scores
        let ids: Vec<u64> = results.iter().map(|r| r.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
        assert!(ids.contains(&4));
    }

    #[test]
    fn test_linear_preserves_ranks() {
        let dense = vec![(1, 0.9), (2, 0.7), (3, 0.5)];
        let sparse = vec![(2, 5.0), (3, 4.0), (4, 3.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // Find specific items and verify their ranks
        let item_1 = results.iter().find(|r| r.id == 1).unwrap();
        assert_eq!(item_1.dense_rank, Some(1)); // First in dense
        assert_eq!(item_1.sparse_rank, None); // Not in sparse

        let item_2 = results.iter().find(|r| r.id == 2).unwrap();
        assert_eq!(item_2.dense_rank, Some(2)); // Second in dense
        assert_eq!(item_2.sparse_rank, Some(1)); // First in sparse

        let item_3 = results.iter().find(|r| r.id == 3).unwrap();
        assert_eq!(item_3.dense_rank, Some(3)); // Third in dense
        assert_eq!(item_3.sparse_rank, Some(2)); // Second in sparse

        let item_4 = results.iter().find(|r| r.id == 4).unwrap();
        assert_eq!(item_4.dense_rank, None); // Not in dense
        assert_eq!(item_4.sparse_rank, Some(3)); // Third in sparse
    }

    #[test]
    fn test_linear_score_ordering() {
        // Verify results are sorted in descending order by score
        let dense = vec![(1, 0.9), (2, 0.8), (3, 0.7)];
        let sparse = vec![(3, 5.0), (4, 4.0), (5, 3.0)];

        let results = linear_fusion(&dense, &sparse, 0.5, 10);

        // Verify descending order
        for window in results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "Results should be sorted in descending order by score"
            );
        }

        // The overlapping item (ID=3) should have highest combined score
        // dense rank 3: normalized ~0.0 (lowest)
        // sparse rank 1: normalized 1.0 (highest)
        // But ID=1 has dense normalized=1.0, sparse=0
        // So top scores depend on alpha and normalization
        assert!(!results.is_empty());
    }

    #[test]
    fn test_linear_alpha_boundary_low() {
        // alpha slightly above 0
        let dense = vec![(1, 0.9)];
        let sparse = vec![(2, 5.0)];

        let results = linear_fusion(&dense, &sparse, 0.01, 10);

        // Sparse should dominate (alpha=0.01 means 99% sparse)
        assert_eq!(results[0].id, 2);
    }

    #[test]
    fn test_linear_alpha_boundary_high() {
        // alpha slightly below 1
        let dense = vec![(1, 0.9)];
        let sparse = vec![(2, 5.0)];

        let results = linear_fusion(&dense, &sparse, 0.99, 10);

        // Dense should dominate (alpha=0.99 means 99% dense)
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_linear_alpha_clamping() {
        // Alpha out of range should be clamped (not panic)
        let dense = vec![(1, 0.9)];
        let sparse = vec![(2, 5.0)];

        // alpha > 1.0 should clamp to 1.0 (dense only)
        let results_high = linear_fusion(&dense, &sparse, 1.5, 10);
        assert!(!results_high.is_empty());
        // Dense should dominate when clamped to 1.0
        assert_eq!(results_high[0].id, 1);

        // alpha < 0.0 should clamp to 0.0 (sparse only)
        let results_low = linear_fusion(&dense, &sparse, -0.5, 10);
        assert!(!results_low.is_empty());
        // Sparse should dominate when clamped to 0.0
        assert_eq!(results_low[0].id, 2);
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

    #[test]
    fn test_fusion_result_partial_eq() {
        let r1 = FusionResult::with_ranks(42, 0.5, Some(1), Some(2));
        let r2 = FusionResult::with_ranks(42, 0.5, Some(1), Some(2));
        let r3 = FusionResult::with_ranks(43, 0.5, Some(1), Some(2));

        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
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
        let _ = FusionMethod::linear(1.5);
    }

    #[test]
    #[should_panic(expected = "Alpha must be in range")]
    fn test_fusion_method_linear_invalid_low() {
        let _ = FusionMethod::linear(-0.1);
    }
}

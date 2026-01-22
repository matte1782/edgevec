//! Hybrid search combining dense and sparse retrieval.
//!
//! This module provides the `HybridSearcher` that orchestrates HNSW index
//! search and sparse storage search, then fuses results using RRF or
//! linear combination.

use crate::hnsw::{GraphError, HnswIndex, VectorId};
use crate::sparse::{SparseSearcher, SparseStorage, SparseVector};
use crate::storage::VectorStorage;

use super::fusion::{linear_fusion, rrf_fusion, FusionMethod, FusionResult};

// =============================================================================
// HYBRID ERROR
// =============================================================================

/// Errors that can occur during hybrid search.
#[derive(Debug, Clone, PartialEq)]
pub enum HybridError {
    /// Configuration validation failed.
    InvalidConfig(String),
    /// Dense search failed.
    DenseSearchError(String),
    /// Dimension mismatch between query and index.
    DimensionMismatch {
        /// Expected dimensions.
        expected: usize,
        /// Actual dimensions.
        actual: usize,
    },
}

impl std::fmt::Display for HybridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HybridError::InvalidConfig(msg) => write!(f, "Invalid config: {msg}"),
            HybridError::DenseSearchError(msg) => write!(f, "Dense search error: {msg}"),
            HybridError::DimensionMismatch { expected, actual } => {
                write!(f, "Dimension mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

impl std::error::Error for HybridError {}

impl From<GraphError> for HybridError {
    fn from(err: GraphError) -> Self {
        match err {
            GraphError::DimensionMismatch { expected, actual } => {
                HybridError::DimensionMismatch { expected, actual }
            }
            other => HybridError::DenseSearchError(other.to_string()),
        }
    }
}

// =============================================================================
// HYBRID SEARCH CONFIG
// =============================================================================

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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Both `dense_k` and `sparse_k` are 0
    /// - `final_k` is 0
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

// =============================================================================
// HYBRID SEARCH RESULT
// =============================================================================

/// Result from hybrid search.
///
/// Contains the fused ranking with optional original ranks and scores.
///
/// # Note on Equality
///
/// This type derives `PartialEq` which compares `f32` scores directly.
/// Due to floating-point precision, two results with nearly-identical
/// scores may not compare equal. For score comparison, consider using
/// an epsilon-based comparison instead of direct equality.
#[derive(Clone, Debug, PartialEq)]
pub struct HybridSearchResult {
    /// Vector ID.
    pub id: VectorId,
    /// Combined score from fusion.
    pub score: f32,
    /// Original rank in dense results (1-indexed, None if not found).
    pub dense_rank: Option<usize>,
    /// Original dense similarity score (None if not found).
    pub dense_score: Option<f32>,
    /// Original rank in sparse results (1-indexed, None if not found).
    pub sparse_rank: Option<usize>,
    /// Original sparse score (None if not found).
    pub sparse_score: Option<f32>,
}

impl HybridSearchResult {
    /// Create from fusion result with original score lookups.
    ///
    /// Uses pre-built HashMaps for O(1) score lookup instead of O(n) linear search.
    fn from_fusion(
        fusion: &FusionResult,
        dense_scores: &std::collections::HashMap<u64, f32>,
        sparse_scores: &std::collections::HashMap<u64, f32>,
    ) -> Self {
        Self {
            id: VectorId(fusion.id),
            score: fusion.score,
            dense_rank: fusion.dense_rank,
            dense_score: dense_scores.get(&fusion.id).copied(),
            sparse_rank: fusion.sparse_rank,
            sparse_score: sparse_scores.get(&fusion.id).copied(),
        }
    }
}

// =============================================================================
// HYBRID SEARCHER
// =============================================================================

/// Hybrid search combining dense and sparse retrieval.
///
/// Orchestrates HNSW index search and sparse storage search,
/// then fuses results using the configured method.
///
/// # ID Alignment Contract
///
/// **IMPORTANT**: The caller is responsible for ensuring that the same document
/// has matching numeric IDs in both dense and sparse storage:
///
/// 1. **Insert Order Alignment**: Insert dense and sparse vectors in the same
///    order, so auto-assigned IDs match (recommended for simple use cases).
///
/// 2. **Explicit ID Management**: Use `insert_with_id()` methods (when available)
///    to explicitly set matching IDs for the same document.
///
/// 3. **Mapping Layer**: Maintain an external document_id -> (VectorId, SparseId)
///    mapping if IDs cannot be aligned at insert time.
///
/// If IDs don't align, fusion will treat them as different documents, and
/// hybrid search quality will degrade significantly.
///
/// # Example
///
/// ```rust,ignore
/// use edgevec::hybrid::{HybridSearcher, HybridSearchConfig};
/// use edgevec::sparse::SparseVector;
///
/// // Assume index, dense_storage, sparse_storage are set up
/// let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);
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
/// ```
pub struct HybridSearcher<'a> {
    index: &'a HnswIndex,
    dense_storage: &'a VectorStorage,
    sparse_storage: &'a SparseStorage,
}

impl<'a> HybridSearcher<'a> {
    /// Create a new hybrid searcher.
    ///
    /// # Arguments
    ///
    /// * `index` - HNSW index for dense search
    /// * `dense_storage` - Vector storage for dense vectors
    /// * `sparse_storage` - Sparse vector storage
    ///
    /// # Note
    ///
    /// If `dense_storage.len() != sparse_storage.len()`, this may indicate
    /// ID misalignment between dense and sparse vectors. Consider verifying
    /// that vectors were inserted in the same order.
    #[must_use]
    pub fn new(
        index: &'a HnswIndex,
        dense_storage: &'a VectorStorage,
        sparse_storage: &'a SparseStorage,
    ) -> Self {
        // Optional: warn if lengths differ (suggests misalignment)
        #[cfg(debug_assertions)]
        if dense_storage.len() != sparse_storage.len() {
            eprintln!(
                "[HybridSearcher] Warning: dense_storage.len()={} != sparse_storage.len()={}. \
                 This may indicate ID misalignment.",
                dense_storage.len(),
                sparse_storage.len()
            );
        }

        Self {
            index,
            dense_storage,
            sparse_storage,
        }
    }

    /// Perform hybrid search combining dense and sparse retrieval.
    ///
    /// # Algorithm
    ///
    /// 1. Execute dense search via HNSW index
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
    /// - Dense query dimension doesn't match index
    /// - Config validation fails
    pub fn search(
        &self,
        dense_query: &[f32],
        sparse_query: &SparseVector,
        config: &HybridSearchConfig,
    ) -> Result<Vec<HybridSearchResult>, HybridError> {
        // Validate config
        config.validate().map_err(HybridError::InvalidConfig)?;

        // Execute dense search
        let dense_results = if config.dense_k > 0 {
            self.index
                .search(dense_query, config.dense_k, self.dense_storage)?
                .into_iter()
                .map(|r| (r.vector_id.0, r.distance))
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

        // Build score lookup maps for O(1) access
        let dense_scores: std::collections::HashMap<u64, f32> =
            dense_results.iter().copied().collect();
        let sparse_scores: std::collections::HashMap<u64, f32> =
            sparse_results.iter().copied().collect();

        // Convert to HybridSearchResult
        let results = fused
            .iter()
            .map(|f| HybridSearchResult::from_fusion(f, &dense_scores, &sparse_scores))
            .collect();

        Ok(results)
    }

    /// Search with dense-only (sparse disabled).
    ///
    /// Useful for A/B testing or when sparse features aren't available.
    ///
    /// # Arguments
    ///
    /// * `dense_query` - Dense embedding vector for HNSW search
    /// * `k` - Number of results to return
    ///
    /// # Errors
    ///
    /// Returns error if dense query dimension doesn't match index.
    pub fn search_dense_only(
        &self,
        dense_query: &[f32],
        k: usize,
    ) -> Result<Vec<HybridSearchResult>, HybridError> {
        let config = HybridSearchConfig {
            dense_k: k,
            sparse_k: 0,
            final_k: k,
            fusion: FusionMethod::rrf(), // Doesn't matter for dense-only
        };

        // Sparse query doesn't matter, use empty-ish vector
        let sparse_query = SparseVector::singleton(0, 0.0, 1)
            .map_err(|e| HybridError::InvalidConfig(e.to_string()))?;

        self.search(dense_query, &sparse_query, &config)
    }

    /// Search with sparse-only (dense disabled).
    ///
    /// Useful for keyword-only search or A/B testing.
    ///
    /// # Arguments
    ///
    /// * `sparse_query` - Sparse vector for keyword search
    /// * `k` - Number of results to return
    ///
    /// # Errors
    ///
    /// Returns error if internal search fails (should not happen in normal use).
    pub fn search_sparse_only(
        &self,
        sparse_query: &SparseVector,
        k: usize,
    ) -> Result<Vec<HybridSearchResult>, HybridError> {
        let config = HybridSearchConfig {
            dense_k: 0,
            sparse_k: k,
            final_k: k,
            fusion: FusionMethod::rrf(), // Doesn't matter for sparse-only
        };

        // Dense query doesn't matter, use zeros
        let dense_query = vec![0.0; self.dense_storage.dimensions() as usize];

        self.search(&dense_query, sparse_query, &config)
    }

    /// Get references to underlying components.
    #[must_use]
    pub fn components(&self) -> (&HnswIndex, &VectorStorage, &SparseStorage) {
        (self.index, self.dense_storage, self.sparse_storage)
    }

    /// Get number of vectors in dense storage.
    #[must_use]
    pub fn dense_count(&self) -> usize {
        self.dense_storage.len()
    }

    /// Get number of vectors in sparse storage.
    #[must_use]
    pub fn sparse_count(&self) -> usize {
        self.sparse_storage.len()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ============= HybridSearchConfig Tests =============

    #[test]
    fn test_config_default() {
        let config = HybridSearchConfig::default();
        assert_eq!(config.dense_k, 20);
        assert_eq!(config.sparse_k, 20);
        assert_eq!(config.final_k, 10);
    }

    #[test]
    fn test_config_rrf() {
        let config = HybridSearchConfig::rrf(30, 40, 15);
        assert_eq!(config.dense_k, 30);
        assert_eq!(config.sparse_k, 40);
        assert_eq!(config.final_k, 15);
        assert!(matches!(config.fusion, FusionMethod::Rrf { k: 60 }));
    }

    #[test]
    fn test_config_rrf_with_k() {
        let config = HybridSearchConfig::rrf_with_k(20, 20, 10, 100);
        assert!(matches!(config.fusion, FusionMethod::Rrf { k: 100 }));
    }

    #[test]
    fn test_config_linear() {
        let config = HybridSearchConfig::linear(20, 20, 10, 0.7);
        match config.fusion {
            FusionMethod::Linear { alpha } => assert!((alpha - 0.7).abs() < 1e-6),
            _ => panic!("Expected Linear fusion"),
        }
    }

    #[test]
    fn test_config_validation_both_zero() {
        let config = HybridSearchConfig {
            dense_k: 0,
            sparse_k: 0,
            final_k: 10,
            fusion: FusionMethod::rrf(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_final_zero() {
        let config = HybridSearchConfig {
            dense_k: 10,
            sparse_k: 10,
            final_k: 0,
            fusion: FusionMethod::rrf(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = HybridSearchConfig::default();
        assert!(config.validate().is_ok());

        // Dense only is valid
        let config = HybridSearchConfig {
            dense_k: 10,
            sparse_k: 0,
            final_k: 5,
            fusion: FusionMethod::rrf(),
        };
        assert!(config.validate().is_ok());

        // Sparse only is valid
        let config = HybridSearchConfig {
            dense_k: 0,
            sparse_k: 10,
            final_k: 5,
            fusion: FusionMethod::rrf(),
        };
        assert!(config.validate().is_ok());
    }

    // ============= HybridError Tests =============

    #[test]
    fn test_hybrid_error_display() {
        let err = HybridError::InvalidConfig("test".to_string());
        assert!(err.to_string().contains("Invalid config"));

        let err = HybridError::DimensionMismatch {
            expected: 128,
            actual: 64,
        };
        assert!(err.to_string().contains("128"));
        assert!(err.to_string().contains("64"));
    }

    #[test]
    fn test_hybrid_error_from_graph_error() {
        let graph_err = GraphError::DimensionMismatch {
            expected: 128,
            actual: 64,
        };
        let hybrid_err: HybridError = graph_err.into();
        assert!(matches!(
            hybrid_err,
            HybridError::DimensionMismatch {
                expected: 128,
                actual: 64
            }
        ));
    }

    // ============= HybridSearchResult Tests =============

    #[test]
    fn test_result_from_fusion() {
        use std::collections::HashMap;

        let fusion = FusionResult::with_ranks(42, 0.5, Some(1), Some(2));
        let dense_scores: HashMap<u64, f32> = [(42, 0.9), (100, 0.8)].into_iter().collect();
        let sparse_scores: HashMap<u64, f32> = [(42, 5.0), (200, 4.0)].into_iter().collect();

        let result = HybridSearchResult::from_fusion(&fusion, &dense_scores, &sparse_scores);

        assert_eq!(result.id.0, 42);
        assert_eq!(result.score, 0.5);
        assert_eq!(result.dense_rank, Some(1));
        assert_eq!(result.sparse_rank, Some(2));
        assert_eq!(result.dense_score, Some(0.9));
        assert_eq!(result.sparse_score, Some(5.0));
    }

    #[test]
    fn test_result_from_fusion_missing_scores() {
        use std::collections::HashMap;

        let fusion = FusionResult::with_ranks(42, 0.5, Some(1), None);
        let dense_scores: HashMap<u64, f32> = [(42, 0.9)].into_iter().collect();
        let sparse_scores: HashMap<u64, f32> = HashMap::new();

        let result = HybridSearchResult::from_fusion(&fusion, &dense_scores, &sparse_scores);

        assert_eq!(result.dense_score, Some(0.9));
        assert_eq!(result.sparse_score, None);
    }
}

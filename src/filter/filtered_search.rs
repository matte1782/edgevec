//! Filtered search integration for EdgeVec.
//!
//! This module provides filtered search capabilities by combining HNSW search
//! with filter expression evaluation. It implements three strategies:
//!
//! - **PreFilter**: Scan all metadata first, then search on matching subset
//! - **PostFilter**: Search with oversampling, then filter results
//! - **Hybrid**: Estimate selectivity and adapt strategy dynamically
//!
//! # Architecture
//!
//! This module acts as an integration layer between:
//! - `HnswIndex` (graph search)
//! - `VectorStorage` (vector data)
//! - `MetadataStore` (filter evaluation)
//!
//! # Design Choice: Wrapper Pattern vs Direct HnswIndex Method
//!
//! We chose the **wrapper pattern** (`FilteredSearcher`) over adding a
//! `search_filtered()` method directly to `HnswIndex` for several reasons:
//!
//! 1. **Separation of Concerns**: `HnswIndex` is focused on graph operations
//!    (insert, delete, search). Filtering is a cross-cutting concern that
//!    involves metadata, which is outside the graph's responsibility.
//!
//! 2. **Flexibility**: The wrapper can work with any `MetadataStore`
//!    implementation, allowing users to provide custom metadata backends
//!    (e.g., in-memory, SQLite, Redis) without modifying the core index.
//!
//! 3. **No Lifetime Coupling**: Adding metadata to `HnswIndex` would require
//!    additional lifetime parameters or owned data, complicating the API.
//!    The wrapper composes references at call time, keeping lifetimes local.
//!
//! 4. **Optional Feature**: Not all users need filtering. The wrapper pattern
//!    keeps the core index lean for users who only need vector search.
//!
//! 5. **Testability**: The wrapper can be tested independently with mock
//!    implementations of `MetadataStore`, without setting up a full index.
//!
//! The tradeoff is slightly more verbose API (`FilteredSearcher::new()` vs
//! `index.search_filtered()`), but the benefits outweigh this cost.
//!
//! # Example
//!
//! ```rust,ignore
//! use edgevec::filter::{parse, FilteredSearcher, FilterStrategy};
//!
//! let searcher = FilteredSearcher::new(&index, &storage, &metadata);
//! let filter = parse("category = \"gpu\" AND price < 500")?;
//! let results = searcher.search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)?;
//! ```

use crate::filter::ast::FilterExpr;
use crate::filter::error::FilterError;
use crate::filter::evaluator::evaluate;
use crate::filter::strategy::{
    calculate_oversample, estimate_selectivity, is_contradiction, is_tautology, select_strategy,
    FilterStrategy, MetadataStore, EF_CAP,
};
use crate::hnsw::graph::{GraphError, HnswIndex};
use crate::hnsw::search::{SearchContext, SearchResult};
use crate::metadata::MetadataValue;
use crate::storage::VectorStorage;
use std::collections::{HashMap, HashSet};

// ═══════════════════════════════════════════════════════════════════════════════
// FILTERED SEARCH RESULT
// ═══════════════════════════════════════════════════════════════════════════════

/// Result from a filtered search operation.
///
/// Contains the search results along with diagnostic information about
/// the search process, including which strategy was used and how many
/// vectors were evaluated.
///
/// # Fields
///
/// - `results`: The actual search results (may be fewer than k if filter is restrictive)
/// - `complete`: Whether the full k results were found
/// - `observed_selectivity`: Fraction of candidates that passed the filter
/// - `strategy_used`: The strategy that was actually executed
/// - `vectors_evaluated`: Total number of vectors checked during search
#[derive(Debug, Clone)]
pub struct FilteredSearchResult {
    /// Search results (may be fewer than k if filter is restrictive).
    pub results: Vec<SearchResult>,
    /// Whether the full k results were found.
    pub complete: bool,
    /// Observed selectivity (fraction of candidates that passed).
    pub observed_selectivity: f32,
    /// Strategy actually used for this query.
    pub strategy_used: FilterStrategy,
    /// Number of vectors evaluated.
    pub vectors_evaluated: usize,
}

impl FilteredSearchResult {
    /// Create an empty result (used for contradictions or empty index).
    #[must_use]
    pub fn empty(strategy: FilterStrategy) -> Self {
        Self {
            results: vec![],
            complete: true,
            observed_selectivity: 0.0,
            strategy_used: strategy,
            vectors_evaluated: 0,
        }
    }

    /// Create a result indicating all vectors matched (tautology case).
    #[must_use]
    pub fn full(results: Vec<SearchResult>, strategy: FilterStrategy) -> Self {
        let len = results.len();
        Self {
            results,
            complete: true,
            observed_selectivity: 1.0,
            strategy_used: strategy,
            vectors_evaluated: len,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FILTERED SEARCH ERROR
// ═══════════════════════════════════════════════════════════════════════════════

/// Error type for filtered search operations.
#[derive(Debug, Clone)]
pub enum FilteredSearchError {
    /// Filter-related error (parsing, evaluation, strategy).
    Filter(FilterError),
    /// Graph-related error (search, dimension mismatch).
    Graph(GraphError),
}

impl From<FilterError> for FilteredSearchError {
    fn from(e: FilterError) -> Self {
        FilteredSearchError::Filter(e)
    }
}

impl From<GraphError> for FilteredSearchError {
    fn from(e: GraphError) -> Self {
        FilteredSearchError::Graph(e)
    }
}

impl std::fmt::Display for FilteredSearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilteredSearchError::Filter(e) => write!(f, "filter error: {e}"),
            FilteredSearchError::Graph(e) => write!(f, "graph error: {e}"),
        }
    }
}

impl std::error::Error for FilteredSearchError {}

// ═══════════════════════════════════════════════════════════════════════════════
// VECTOR METADATA STORE IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════════

/// A simple metadata store backed by a Vec of HashMaps.
///
/// This is the default implementation of `MetadataStore` for filtered search.
/// Vector IDs map directly to indices in the metadata vector.
#[derive(Debug, Clone, Default)]
pub struct VectorMetadataStore {
    /// Metadata for each vector, indexed by vector position (0-based).
    metadata: Vec<HashMap<String, MetadataValue>>,
}

impl VectorMetadataStore {
    /// Create a new empty metadata store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
        }
    }

    /// Create a metadata store with pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            metadata: Vec::with_capacity(capacity),
        }
    }

    /// Add metadata for a new vector.
    ///
    /// Returns the index of the added metadata.
    pub fn push(&mut self, metadata: HashMap<String, MetadataValue>) -> usize {
        let idx = self.metadata.len();
        self.metadata.push(metadata);
        idx
    }

    /// Set metadata for a specific index.
    ///
    /// Grows the vector if necessary, filling gaps with empty HashMaps.
    pub fn set(&mut self, idx: usize, metadata: HashMap<String, MetadataValue>) {
        if idx >= self.metadata.len() {
            self.metadata.resize_with(idx + 1, HashMap::new);
        }
        self.metadata[idx] = metadata;
    }

    /// Get metadata by index.
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&HashMap<String, MetadataValue>> {
        self.metadata.get(idx)
    }
}

impl MetadataStore for VectorMetadataStore {
    fn get_metadata(&self, id: usize) -> Option<&HashMap<String, MetadataValue>> {
        self.metadata.get(id)
    }

    fn len(&self) -> usize {
        self.metadata.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FILTERED SEARCHER
// ═══════════════════════════════════════════════════════════════════════════════

/// Filtered search executor combining HNSW search with filter evaluation.
///
/// This struct provides the main `search_filtered()` API that integrates
/// filter expressions with HNSW search using configurable strategies.
///
/// # Lifetime Parameters
///
/// - `'idx`: Lifetime of the HNSW index reference
/// - `'sto`: Lifetime of the vector storage reference
/// - `'meta`: Lifetime of the metadata store reference
///
/// # Example
///
/// ```rust,ignore
/// let searcher = FilteredSearcher::new(&index, &storage, &metadata);
/// let results = searcher.search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)?;
/// ```
pub struct FilteredSearcher<'idx, 'sto, 'meta, M: MetadataStore> {
    index: &'idx HnswIndex,
    storage: &'sto VectorStorage,
    metadata: &'meta M,
    /// Reusable search context for better performance.
    search_ctx: SearchContext,
}

impl<'idx, 'sto, 'meta, M: MetadataStore> FilteredSearcher<'idx, 'sto, 'meta, M> {
    /// Create a new filtered searcher.
    ///
    /// # Arguments
    ///
    /// * `index` - The HNSW index to search
    /// * `storage` - Vector storage for distance calculations
    /// * `metadata` - Metadata store for filter evaluation
    #[must_use]
    pub fn new(index: &'idx HnswIndex, storage: &'sto VectorStorage, metadata: &'meta M) -> Self {
        Self {
            index,
            storage,
            metadata,
            search_ctx: SearchContext::new(),
        }
    }

    /// Search with optional filter and strategy.
    ///
    /// # Arguments
    ///
    /// * `query` - Query vector (same dimensions as indexed vectors)
    /// * `k` - Number of results to return
    /// * `filter` - Optional filter expression (None = no filtering)
    /// * `strategy` - Filter strategy (Auto for automatic selection)
    ///
    /// # Returns
    ///
    /// * `Ok(FilteredSearchResult)` - Search results with diagnostics
    /// * `Err(FilteredSearchError)` - On invalid filter or search failure
    ///
    /// # Errors
    ///
    /// Returns `FilteredSearchError::Filter` if:
    /// - Filter strategy validation fails
    /// - Filter expression evaluation fails
    ///
    /// Returns `FilteredSearchError::Graph` if:
    /// - Query dimension mismatches index
    /// - HNSW search fails
    ///
    /// # Performance
    ///
    /// - Auto strategy: Estimates selectivity, chooses best approach
    /// - PreFilter: O(n) metadata scan + O(log m) search on m matches
    /// - PostFilter: O(log n × ef_search) with oversampling
    /// - Hybrid: Adaptive oversampling based on estimated selectivity
    pub fn search_filtered(
        &mut self,
        query: &[f32],
        k: usize,
        filter: Option<&FilterExpr>,
        strategy: FilterStrategy,
    ) -> Result<FilteredSearchResult, FilteredSearchError> {
        // Validate strategy
        strategy.validate()?;

        // No filter = standard search
        let Some(filter) = filter else {
            let results = self.index.search(query, k, self.storage)?;
            return Ok(FilteredSearchResult {
                complete: results.len() >= k || self.index.is_empty(),
                vectors_evaluated: k.min(self.index.len()),
                observed_selectivity: 1.0,
                strategy_used: strategy,
                results,
            });
        };

        // Handle edge cases (tautology, contradiction, empty index)
        if let Some(result) = self.handle_filter_edge_cases(filter, k, query)? {
            return Ok(result);
        }

        // Determine actual strategy (resolve Auto)
        let actual_strategy = match strategy {
            FilterStrategy::Auto => {
                let estimate = estimate_selectivity(filter, self.metadata, Some(42));
                select_strategy(estimate.selectivity)
            }
            other => other,
        };

        // Execute appropriate strategy
        match actual_strategy {
            FilterStrategy::PreFilter => self.search_prefilter(query, k, filter),
            FilterStrategy::PostFilter { oversample } => {
                self.search_postfilter(query, k, filter, oversample)
            }
            FilterStrategy::Hybrid {
                oversample_min,
                oversample_max,
            } => self.search_hybrid(query, k, filter, oversample_min, oversample_max),
            FilterStrategy::Auto => unreachable!("Auto already resolved above"),
        }
    }

    /// Handle filter edge cases before executing search.
    ///
    /// Returns `Some(result)` if edge case handled, `None` to proceed normally.
    fn handle_filter_edge_cases(
        &mut self,
        filter: &FilterExpr,
        k: usize,
        query: &[f32],
    ) -> Result<Option<FilteredSearchResult>, FilteredSearchError> {
        // Empty index
        if self.index.is_empty() {
            return Ok(Some(FilteredSearchResult::empty(FilterStrategy::Auto)));
        }

        // Check for tautology (always true) - proceed without filter
        if is_tautology(filter) {
            let results = self.index.search(query, k, self.storage)?;
            return Ok(Some(FilteredSearchResult::full(
                results,
                FilterStrategy::Auto,
            )));
        }

        // Check for contradiction (always false) - return empty
        if is_contradiction(filter) {
            return Ok(Some(FilteredSearchResult::empty(FilterStrategy::Auto)));
        }

        Ok(None) // No edge case, proceed normally
    }

    /// PreFilter strategy: scan all metadata, then search on matching subset.
    ///
    /// This strategy is efficient when selectivity is high (>80% match).
    ///
    /// # Performance
    ///
    /// - Metadata scan: O(n) where n = total vectors
    /// - Result filtering: O(k) using HashSet for O(1) lookups
    /// - Total: O(n + k × log(n)) where log(n) is HNSW search complexity
    fn search_prefilter(
        &mut self,
        query: &[f32],
        k: usize,
        filter: &FilterExpr,
    ) -> Result<FilteredSearchResult, FilteredSearchError> {
        // Build HashSet of passing vector indices for O(1) lookup
        // [m1 FIX] Changed from Vec to HashSet for performance
        let mut passing_indices = HashSet::new();
        let total = self.metadata.len();

        for idx in 0..total {
            if let Some(metadata) = self.metadata.get_metadata(idx) {
                if evaluate(filter, metadata).unwrap_or(false) {
                    passing_indices.insert(idx);
                }
            }
        }

        let passed = passing_indices.len();
        #[allow(clippy::cast_precision_loss)]
        let selectivity = if total > 0 {
            (passed as f32) / (total as f32)
        } else {
            0.0
        };

        if passing_indices.is_empty() {
            return Ok(FilteredSearchResult {
                results: vec![],
                complete: true,
                observed_selectivity: 0.0,
                strategy_used: FilterStrategy::PreFilter,
                vectors_evaluated: total,
            });
        }

        // Search on full index and filter results
        // Note: True pre-filter would search only on subset, but HNSW
        // doesn't support masked search. We approximate by searching
        // with larger ef and filtering.
        let ef_effective = (k * 10).min(EF_CAP).max(k);
        let all_results = self.index.search_with_context(
            query,
            ef_effective,
            self.storage,
            &mut self.search_ctx,
        )?;

        // Filter to only passing indices - O(1) per lookup with HashSet
        let mut results = Vec::with_capacity(k);
        for result in all_results {
            if results.len() >= k {
                break;
            }
            // Convert VectorId to index (VectorId starts at 1, index at 0)
            // Note: VectorId is u64, index is usize. On 32-bit systems this may truncate,
            // but we don't support indices > u32::MAX anyway.
            #[allow(clippy::cast_possible_truncation)]
            let idx = (result.vector_id.0 as usize).saturating_sub(1);
            if passing_indices.contains(&idx) {
                results.push(result);
            }
        }

        Ok(FilteredSearchResult {
            complete: results.len() >= k,
            observed_selectivity: selectivity,
            strategy_used: FilterStrategy::PreFilter,
            vectors_evaluated: total,
            results,
        })
    }

    /// PostFilter strategy: search with oversampling, then filter results.
    ///
    /// This strategy is efficient when selectivity is low (<5% match).
    fn search_postfilter(
        &mut self,
        query: &[f32],
        k: usize,
        filter: &FilterExpr,
        oversample: f32,
    ) -> Result<FilteredSearchResult, FilteredSearchError> {
        // Calculate oversampled search size
        // Note: k is typically small (10-100), precision loss is acceptable
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_precision_loss)]
        let ef_effective = ((k as f32) * oversample).ceil() as usize;
        let ef_effective = ef_effective.min(EF_CAP).max(k);

        // Run HNSW search with oversampled ef
        let candidates = self.index.search_with_context(
            query,
            ef_effective,
            self.storage,
            &mut self.search_ctx,
        )?;

        // Filter candidates
        let mut results = Vec::with_capacity(k);
        let mut passed = 0;
        let evaluated = candidates.len();

        for candidate in candidates {
            if results.len() >= k {
                break;
            }
            // Convert VectorId to index (VectorId starts at 1, index at 0)
            // Note: VectorId is u64, index is usize. On 32-bit systems this may truncate,
            // but we don't support indices > u32::MAX anyway.
            #[allow(clippy::cast_possible_truncation)]
            let idx = (candidate.vector_id.0 as usize).saturating_sub(1);
            if let Some(metadata) = self.metadata.get_metadata(idx) {
                if evaluate(filter, metadata).unwrap_or(false) {
                    results.push(candidate);
                    passed += 1;
                }
            }
        }

        #[allow(clippy::cast_precision_loss)]
        let selectivity = if evaluated > 0 {
            (passed as f32) / (evaluated as f32)
        } else {
            0.0
        };

        Ok(FilteredSearchResult {
            complete: results.len() >= k,
            observed_selectivity: selectivity,
            strategy_used: FilterStrategy::PostFilter { oversample },
            vectors_evaluated: evaluated,
            results,
        })
    }

    /// Hybrid strategy: estimate selectivity, adapt oversample.
    ///
    /// Combines sampling-based selectivity estimation with adaptive oversampling.
    fn search_hybrid(
        &mut self,
        query: &[f32],
        k: usize,
        filter: &FilterExpr,
        oversample_min: f32,
        oversample_max: f32,
    ) -> Result<FilteredSearchResult, FilteredSearchError> {
        // Estimate selectivity
        let estimate = estimate_selectivity(filter, self.metadata, Some(42));

        // Calculate adaptive oversample within bounds
        let oversample = calculate_oversample(estimate.selectivity)
            .max(oversample_min)
            .min(oversample_max);

        // Use post-filter with calculated oversample
        let mut result = self.search_postfilter(query, k, filter, oversample)?;
        result.strategy_used = FilterStrategy::Hybrid {
            oversample_min,
            oversample_max,
        };
        Ok(result)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BINARY FILTERED SEARCH METHODS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Search binary vectors with optional filter and strategy.
    ///
    /// This is the binary vector equivalent of `search_filtered()`, using
    /// Hamming distance for similarity calculation.
    ///
    /// # Arguments
    ///
    /// * `query` - Binary query vector (packed bytes)
    /// * `k` - Number of results to return
    /// * `filter` - Optional filter expression (None = no filtering)
    /// * `strategy` - Filter strategy (Auto for automatic selection)
    ///
    /// # Returns
    ///
    /// * `Ok(FilteredSearchResult)` - Search results with diagnostics
    /// * `Err(FilteredSearchError)` - On invalid filter or search failure
    ///
    /// # Errors
    ///
    /// Returns `FilteredSearchError::Filter` if:
    /// - Filter strategy validation fails
    /// - Filter expression evaluation fails
    ///
    /// Returns `FilteredSearchError::Graph` if:
    /// - Query dimension mismatches index
    /// - Binary HNSW search fails
    pub fn search_binary_filtered(
        &mut self,
        query: &[u8],
        k: usize,
        filter: Option<&FilterExpr>,
        strategy: FilterStrategy,
    ) -> Result<FilteredSearchResult, FilteredSearchError> {
        // Validate strategy
        strategy.validate()?;

        // No filter = standard binary search
        let Some(filter) = filter else {
            let results = self.index.search_binary(query, k, self.storage)?;
            return Ok(FilteredSearchResult {
                complete: results.len() >= k || self.index.is_empty(),
                vectors_evaluated: k.min(self.index.len()),
                observed_selectivity: 1.0,
                strategy_used: strategy,
                results,
            });
        };

        // Handle edge cases (tautology, contradiction, empty index)
        if let Some(result) = self.handle_binary_filter_edge_cases(filter, k, query)? {
            return Ok(result);
        }

        // IMPORTANT: For binary vectors, ALWAYS use PreFilter strategy.
        //
        // Rationale: PostFilter and Hybrid strategies could miss top-K results
        // because binary search returns a fixed candidate set based on Hamming
        // distance. If we filter AFTER search, high-quality matches might be
        // excluded from the initial candidate set.
        //
        // PreFilter ensures all matching vectors are considered before ranking
        // by Hamming distance, guaranteeing correct top-K results.
        //
        // The `strategy` parameter is accepted for API compatibility but ignored
        // for binary search - PreFilter is always used.
        let _ = strategy; // Explicitly ignore - always use PreFilter for binary

        self.search_binary_prefilter(query, k, filter)
    }

    /// Handle binary filter edge cases before executing search.
    fn handle_binary_filter_edge_cases(
        &mut self,
        filter: &FilterExpr,
        k: usize,
        query: &[u8],
    ) -> Result<Option<FilteredSearchResult>, FilteredSearchError> {
        // Empty index
        if self.index.is_empty() {
            return Ok(Some(FilteredSearchResult::empty(FilterStrategy::Auto)));
        }

        // Check for tautology (always true) - proceed without filter
        if is_tautology(filter) {
            let results = self.index.search_binary(query, k, self.storage)?;
            return Ok(Some(FilteredSearchResult::full(
                results,
                FilterStrategy::Auto,
            )));
        }

        // Check for contradiction (always false) - return empty
        if is_contradiction(filter) {
            return Ok(Some(FilteredSearchResult::empty(FilterStrategy::Auto)));
        }

        Ok(None) // No edge case, proceed normally
    }

    /// Binary PreFilter strategy: scan all metadata, then search on matching subset.
    fn search_binary_prefilter(
        &mut self,
        query: &[u8],
        k: usize,
        filter: &FilterExpr,
    ) -> Result<FilteredSearchResult, FilteredSearchError> {
        // Build HashSet of passing vector indices for O(1) lookup
        let mut passing_indices = HashSet::new();
        let total = self.metadata.len();

        for idx in 0..total {
            if let Some(metadata) = self.metadata.get_metadata(idx) {
                if evaluate(filter, metadata).unwrap_or(false) {
                    passing_indices.insert(idx);
                }
            }
        }

        let passed = passing_indices.len();
        #[allow(clippy::cast_precision_loss)]
        let selectivity = if total > 0 {
            (passed as f32) / (total as f32)
        } else {
            0.0
        };

        if passing_indices.is_empty() {
            return Ok(FilteredSearchResult {
                results: vec![],
                complete: true,
                observed_selectivity: 0.0,
                strategy_used: FilterStrategy::PreFilter,
                vectors_evaluated: total,
            });
        }

        // Search on full index and filter results
        let ef_effective = (k * 10).min(EF_CAP).max(k);
        let all_results = self.index.search_binary_with_context(
            query,
            ef_effective,
            self.storage,
            &mut self.search_ctx,
        )?;

        // Filter to only passing indices
        let mut results = Vec::with_capacity(k);
        for result in all_results {
            if results.len() >= k {
                break;
            }
            #[allow(clippy::cast_possible_truncation)]
            let idx = (result.vector_id.0 as usize).saturating_sub(1);
            if passing_indices.contains(&idx) {
                results.push(result);
            }
        }

        Ok(FilteredSearchResult {
            complete: results.len() >= k,
            observed_selectivity: selectivity,
            strategy_used: FilterStrategy::PreFilter,
            vectors_evaluated: total,
            results,
        })
    }

    /// Binary PostFilter strategy: search with oversampling, then filter results.
    ///
    /// NOTE: This function is not currently used because binary search with metadata
    /// filtering always uses PreFilter (see search_binary_filtered). Kept for potential
    /// future use in testing or specialized scenarios.
    #[allow(dead_code)]
    fn search_binary_postfilter(
        &mut self,
        query: &[u8],
        k: usize,
        filter: &FilterExpr,
        oversample: f32,
    ) -> Result<FilteredSearchResult, FilteredSearchError> {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_precision_loss)]
        let ef_effective = ((k as f32) * oversample).ceil() as usize;
        let ef_effective = ef_effective.min(EF_CAP).max(k);

        // Run binary HNSW search with oversampled ef
        let candidates = self.index.search_binary_with_context(
            query,
            ef_effective,
            self.storage,
            &mut self.search_ctx,
        )?;

        // Filter candidates
        let mut results = Vec::with_capacity(k);
        let mut passed = 0;
        let evaluated = candidates.len();

        for candidate in candidates {
            if results.len() >= k {
                break;
            }
            #[allow(clippy::cast_possible_truncation)]
            let idx = (candidate.vector_id.0 as usize).saturating_sub(1);
            if let Some(metadata) = self.metadata.get_metadata(idx) {
                if evaluate(filter, metadata).unwrap_or(false) {
                    results.push(candidate);
                    passed += 1;
                }
            }
        }

        #[allow(clippy::cast_precision_loss)]
        let selectivity = if evaluated > 0 {
            (passed as f32) / (evaluated as f32)
        } else {
            0.0
        };

        Ok(FilteredSearchResult {
            complete: results.len() >= k,
            observed_selectivity: selectivity,
            strategy_used: FilterStrategy::PostFilter { oversample },
            vectors_evaluated: evaluated,
            results,
        })
    }

    /// Binary Hybrid strategy: estimate selectivity, adapt oversample.
    ///
    /// NOTE: This function is not currently used because binary search with metadata
    /// filtering always uses PreFilter (see search_binary_filtered). Kept for potential
    /// future use in testing or specialized scenarios.
    #[allow(dead_code)]
    fn search_binary_hybrid(
        &mut self,
        query: &[u8],
        k: usize,
        filter: &FilterExpr,
        oversample_min: f32,
        oversample_max: f32,
    ) -> Result<FilteredSearchResult, FilteredSearchError> {
        // Estimate selectivity
        let estimate = estimate_selectivity(filter, self.metadata, Some(42));

        // Calculate adaptive oversample within bounds
        let oversample = calculate_oversample(estimate.selectivity)
            .max(oversample_min)
            .min(oversample_max);

        // Use binary post-filter with calculated oversample
        let mut result = self.search_binary_postfilter(query, k, filter, oversample)?;
        result.strategy_used = FilterStrategy::Hybrid {
            oversample_min,
            oversample_max,
        };
        Ok(result)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
#[allow(clippy::float_cmp)] // Tests use exact float comparisons for deterministic values
#[allow(clippy::cast_possible_wrap)] // Test data uses small values that won't wrap
mod tests {
    use super::*;
    use crate::filter::parse;
    use crate::hnsw::config::HnswConfig;
    use crate::hnsw::graph::VectorId;

    /// Create a test index with vectors and metadata.
    fn create_test_index(
        count: usize,
        dim: u32,
    ) -> (HnswIndex, VectorStorage, VectorMetadataStore) {
        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");
        let mut metadata_store = VectorMetadataStore::with_capacity(count);

        for i in 0..count {
            // Create a simple vector
            #[allow(clippy::cast_precision_loss)]
            let vector: Vec<f32> = (0..dim).map(|d| (i + d as usize) as f32).collect();

            // Insert into index (which also stores in storage)
            let _vid = index
                .insert(&vector, &mut storage)
                .expect("Failed to insert into index");

            // Create metadata
            let mut meta = HashMap::new();
            #[allow(clippy::cast_precision_loss)]
            {
                // Alternate categories
                let category = if i % 3 == 0 {
                    "gpu"
                } else if i % 3 == 1 {
                    "cpu"
                } else {
                    "memory"
                };
                meta.insert(
                    "category".to_string(),
                    MetadataValue::String(category.to_string()),
                );
                meta.insert(
                    "price".to_string(),
                    MetadataValue::Integer((i * 100) as i64),
                );
                meta.insert("active".to_string(), MetadataValue::Boolean(i % 2 == 0));
            }
            metadata_store.push(meta);
        }

        (index, storage, metadata_store)
    }

    #[test]
    fn test_search_filtered_no_filter() {
        let (index, storage, metadata) = create_test_index(100, 8);
        let mut searcher = FilteredSearcher::new(&index, &storage, &metadata);

        let query: Vec<f32> = vec![0.0; 8];
        let result = searcher
            .search_filtered(&query, 10, None, FilterStrategy::Auto)
            .unwrap();

        assert_eq!(result.results.len(), 10);
        assert!(result.complete);
        assert!((result.observed_selectivity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_search_filtered_prefilter() {
        let (index, storage, metadata) = create_test_index(100, 8);
        let mut searcher = FilteredSearcher::new(&index, &storage, &metadata);

        let filter = parse("category = \"gpu\"").unwrap();
        let query: Vec<f32> = vec![0.0; 8];
        let result = searcher
            .search_filtered(&query, 10, Some(&filter), FilterStrategy::PreFilter)
            .unwrap();

        assert_eq!(result.strategy_used, FilterStrategy::PreFilter);
        // ~33% are gpu, so we should find some results
        assert!(!result.results.is_empty());
    }

    #[test]
    fn test_search_filtered_postfilter() {
        let (index, storage, metadata) = create_test_index(100, 8);
        let mut searcher = FilteredSearcher::new(&index, &storage, &metadata);

        let filter = parse("category = \"gpu\"").unwrap();
        let query: Vec<f32> = vec![0.0; 8];
        let result = searcher
            .search_filtered(
                &query,
                5,
                Some(&filter),
                FilterStrategy::PostFilter { oversample: 5.0 },
            )
            .unwrap();

        assert!(matches!(
            result.strategy_used,
            FilterStrategy::PostFilter { .. }
        ));
        // Should evaluate at least 5 * 5 = 25 vectors
        assert!(result.vectors_evaluated >= 5);
    }

    #[test]
    fn test_search_filtered_hybrid() {
        let (index, storage, metadata) = create_test_index(100, 8);
        let mut searcher = FilteredSearcher::new(&index, &storage, &metadata);

        let filter = parse("active = true").unwrap();
        let query: Vec<f32> = vec![0.0; 8];
        let result = searcher
            .search_filtered(&query, 5, Some(&filter), FilterStrategy::HYBRID_DEFAULT)
            .unwrap();

        assert!(matches!(
            result.strategy_used,
            FilterStrategy::Hybrid { .. }
        ));
    }

    #[test]
    fn test_search_filtered_auto() {
        let (index, storage, metadata) = create_test_index(100, 8);
        let mut searcher = FilteredSearcher::new(&index, &storage, &metadata);

        // active = true has ~50% selectivity -> should choose Hybrid
        let filter = parse("active = true").unwrap();
        let query: Vec<f32> = vec![0.0; 8];
        let result = searcher
            .search_filtered(&query, 5, Some(&filter), FilterStrategy::Auto)
            .unwrap();

        // Auto should resolve to a concrete strategy
        assert!(!matches!(result.strategy_used, FilterStrategy::Auto));
    }

    #[test]
    fn test_search_filtered_empty_index() {
        let config = HnswConfig::new(8);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::new(config, &storage).expect("Failed to create index");
        let metadata = VectorMetadataStore::new();

        let mut searcher = FilteredSearcher::new(&index, &storage, &metadata);

        let filter = parse("active = true").unwrap();
        let query: Vec<f32> = vec![0.0; 8];
        let result = searcher
            .search_filtered(&query, 5, Some(&filter), FilterStrategy::Auto)
            .unwrap();

        assert!(result.results.is_empty());
        assert!(result.complete);
    }

    #[test]
    fn test_search_filtered_tautology() {
        let (index, storage, metadata) = create_test_index(100, 8);
        let mut searcher = FilteredSearcher::new(&index, &storage, &metadata);

        // TRUE is a tautology
        let filter = FilterExpr::LiteralBool(true);
        let query: Vec<f32> = vec![0.0; 8];
        let result = searcher
            .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
            .unwrap();

        // Should return results without filtering
        assert_eq!(result.results.len(), 10);
        assert!((result.observed_selectivity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_search_filtered_contradiction() {
        let (index, storage, metadata) = create_test_index(100, 8);
        let mut searcher = FilteredSearcher::new(&index, &storage, &metadata);

        // FALSE is a contradiction
        let filter = FilterExpr::LiteralBool(false);
        let query: Vec<f32> = vec![0.0; 8];
        let result = searcher
            .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
            .unwrap();

        // Should return empty immediately
        assert!(result.results.is_empty());
        assert!(result.complete);
        assert_eq!(result.vectors_evaluated, 0);
    }

    #[test]
    fn test_vector_metadata_store() {
        let mut store = VectorMetadataStore::new();

        let mut meta1 = HashMap::new();
        meta1.insert(
            "key".to_string(),
            MetadataValue::String("value1".to_string()),
        );
        store.push(meta1);

        let mut meta2 = HashMap::new();
        meta2.insert(
            "key".to_string(),
            MetadataValue::String("value2".to_string()),
        );
        store.push(meta2);

        assert_eq!(store.len(), 2);
        assert!(store.get(0).is_some());
        assert!(store.get(1).is_some());
        assert!(store.get(2).is_none());
    }

    #[test]
    fn test_filtered_search_result_constructors() {
        let empty = FilteredSearchResult::empty(FilterStrategy::PreFilter);
        assert!(empty.results.is_empty());
        assert!(empty.complete);
        assert_eq!(empty.observed_selectivity, 0.0);

        let results = vec![SearchResult {
            vector_id: VectorId(1),
            distance: 0.5,
        }];
        let full = FilteredSearchResult::full(results.clone(), FilterStrategy::Auto);
        assert_eq!(full.results.len(), 1);
        assert!(full.complete);
        assert_eq!(full.observed_selectivity, 1.0);
    }
}

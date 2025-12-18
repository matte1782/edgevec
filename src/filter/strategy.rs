//! Filter strategy selection for EdgeVec.
//!
//! Determines how filtering integrates with HNSW search based on
//! estimated selectivity and configured parameters.
//!
//! # Strategy Selection
//!
//! | Selectivity | Strategy | Rationale |
//! |:------------|:---------|:----------|
//! | >80% | PreFilter | Most vectors pass; scan all, then search subset |
//! | <5% | PostFilter | Few pass; oversample heavily, filter results |
//! | 5%-80% | Hybrid | Adaptive oversample based on estimated selectivity |
//!
//! # Theoretical Basis
//!
//! Strategy selection is based on the cost model derived from vector database
//! literature (Milvus, Weaviate, Qdrant engineering blogs):
//!
//! ## PreFilter Threshold (80%)
//!
//! When selectivity exceeds 80%, the cost equation favors pre-filtering:
//!
//! ```text
//! Cost_PreFilter = O(N) + O(log(N_filtered))
//! Cost_PostFilter = O(log(N)) + O(k × oversample)
//!
//! When selectivity > 0.8:
//!   N_filtered ≈ 0.8N, so PreFilter cost ≈ O(N) + O(log(0.8N))
//!   PostFilter would need oversample ≈ 1/0.8 = 1.25x (low benefit)
//!   → Full metadata scan amortizes well when most vectors match
//! ```
//!
//! ## PostFilter Threshold (5%)
//!
//! When selectivity drops below 5%, oversample approaches practical limits:
//!
//! ```text
//! Required oversample ≈ 1 / selectivity
//! At 5% selectivity: oversample = 20x (exceeds MAX_OVERSAMPLE)
//! At 10% selectivity: oversample = 10x (at MAX_OVERSAMPLE cap)
//! At 3% selectivity: oversample = 33x (capped, recall degradation)
//!
//! → Below 5%, aggressive post-filtering with max oversample is used
//! → Recall may degrade; user should consider PreFilter for precision
//! ```
//!
//! ## Oversample Calculation
//!
//! The oversample factor is derived from the probability model:
//!
//! ```text
//! P(finding k matches in k×o candidates) = 1 - (1-s)^(k×o)
//!
//! For 95% confidence with selectivity s:
//!   oversample ≈ min(1/s, MAX_OVERSAMPLE)
//!
//! This ensures high probability of finding k matching vectors
//! while bounding latency via MAX_OVERSAMPLE cap.
//! ```
//!
//! ## References
//!
//! - Milvus: "Filtered Vector Search" technical blog
//! - Qdrant: "Payload Indexing" documentation
//! - Weaviate: "Filters in Vector Search" engineering notes
//!
//! # Example
//!
//! ```rust
//! use edgevec::filter::strategy::{FilterStrategy, select_strategy, calculate_oversample};
//!
//! // High selectivity (90% pass) -> PreFilter
//! assert_eq!(select_strategy(0.9), FilterStrategy::PreFilter);
//!
//! // Low selectivity (3% pass) -> PostFilter with high oversample
//! let strategy = select_strategy(0.03);
//! assert!(matches!(strategy, FilterStrategy::PostFilter { .. }));
//!
//! // Medium selectivity (30% pass) -> Hybrid
//! let strategy = select_strategy(0.3);
//! assert!(matches!(strategy, FilterStrategy::Hybrid { .. }));
//! ```

use crate::filter::ast::FilterExpr;
use crate::filter::error::FilterError;
use crate::filter::evaluator::evaluate;
use crate::metadata::MetadataValue;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Maximum oversample factor to prevent ef explosion.
///
/// Even with very low selectivity, we cap oversample at 10x to bound latency.
pub const MAX_OVERSAMPLE: f32 = 10.0;

/// Default oversample when selectivity is unknown.
///
/// 3x is a reasonable default that balances recall and performance.
pub const DEFAULT_OVERSAMPLE: f32 = 3.0;

/// Absolute cap on ef_search to bound latency.
///
/// Regardless of oversample, ef_search never exceeds this value.
pub const EF_CAP: usize = 1000;

/// Minimum sample size for selectivity estimation.
///
/// Sampling 100 vectors provides reasonable estimation accuracy while
/// keeping overhead low.
pub const SELECTIVITY_SAMPLE_SIZE: usize = 100;

/// Selectivity threshold above which pre-filter is preferred.
///
/// When >80% of vectors pass the filter, it's more efficient to scan all
/// metadata first, then search only the passing subset.
pub const PREFILTER_THRESHOLD: f32 = 0.8;

/// Selectivity threshold below which post-filter is sufficient.
///
/// When <5% of vectors pass, we use high oversample with post-filtering.
pub const POSTFILTER_THRESHOLD: f32 = 0.05;

// ═══════════════════════════════════════════════════════════════════════════════
// FILTER STRATEGY ENUM
// ═══════════════════════════════════════════════════════════════════════════════

/// Strategy for combining filtering with HNSW search.
///
/// # Variants
///
/// | Variant | Description | When to Use |
/// |:--------|:------------|:------------|
/// | `PostFilter` | Search first, filter results | Low selectivity (<5%) |
/// | `PreFilter` | Filter first, search subset | High selectivity (>80%) |
/// | `Hybrid` | Adaptive oversample | Medium selectivity |
/// | `Auto` | Automatic selection | Default; estimates selectivity |
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FilterStrategy {
    /// Post-filter with fixed oversample factor.
    ///
    /// Retrieves `k × oversample` candidates from HNSW, then filters.
    /// Best for low selectivity (few vectors pass filter).
    PostFilter {
        /// Oversample factor (1.0 = no oversampling, 10.0 = max).
        oversample: f32,
    },

    /// Pre-filter (full metadata scan, then search on subset).
    ///
    /// Scans all metadata to find matching vectors, then searches
    /// only the matching subset. Best for high selectivity (most pass).
    PreFilter,

    /// Hybrid with bounded oversample (adaptive based on selectivity).
    ///
    /// Estimates selectivity via sampling, then calculates appropriate
    /// oversample within the given bounds.
    Hybrid {
        /// Minimum oversample (floor).
        oversample_min: f32,
        /// Maximum oversample (ceiling).
        oversample_max: f32,
    },

    /// Automatic strategy selection based on estimated selectivity.
    ///
    /// Samples vectors to estimate selectivity, then selects the
    /// appropriate strategy (PreFilter, PostFilter, or Hybrid).
    #[default]
    Auto,
}

impl FilterStrategy {
    /// Post-filter with default oversample (3x).
    pub const POST_FILTER_DEFAULT: Self = FilterStrategy::PostFilter {
        oversample: DEFAULT_OVERSAMPLE,
    };

    /// Hybrid with default bounds (1.5x to 10x).
    pub const HYBRID_DEFAULT: Self = FilterStrategy::Hybrid {
        oversample_min: 1.5,
        oversample_max: MAX_OVERSAMPLE,
    };

    /// Validate strategy configuration.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError::InvalidStrategy`] if:
    /// - `PostFilter.oversample` < 1.0 or > `MAX_OVERSAMPLE`
    /// - `Hybrid.oversample_min` < 1.0
    /// - `Hybrid.oversample_max` < `oversample_min`
    /// - `Hybrid.oversample_max` > `MAX_OVERSAMPLE`
    pub fn validate(&self) -> Result<(), FilterError> {
        match self {
            FilterStrategy::PostFilter { oversample } => {
                if *oversample < 1.0 {
                    return Err(FilterError::InvalidStrategy(
                        "oversample must be >= 1.0".into(),
                    ));
                }
                if *oversample > MAX_OVERSAMPLE {
                    return Err(FilterError::InvalidStrategy(format!(
                        "oversample must be <= {MAX_OVERSAMPLE}"
                    )));
                }
                Ok(())
            }
            FilterStrategy::Hybrid {
                oversample_min,
                oversample_max,
            } => {
                if *oversample_min < 1.0 {
                    return Err(FilterError::InvalidStrategy(
                        "oversample_min must be >= 1.0".into(),
                    ));
                }
                if *oversample_max < *oversample_min {
                    return Err(FilterError::InvalidStrategy(
                        "oversample_max must be >= oversample_min".into(),
                    ));
                }
                if *oversample_max > MAX_OVERSAMPLE {
                    return Err(FilterError::InvalidStrategy(format!(
                        "oversample_max must be <= {MAX_OVERSAMPLE}"
                    )));
                }
                Ok(())
            }
            FilterStrategy::PreFilter | FilterStrategy::Auto => Ok(()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SELECTIVITY ESTIMATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Selectivity estimation result.
///
/// Contains both the estimated selectivity and diagnostic information
/// about the sampling process.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectivityEstimate {
    /// Estimated fraction of vectors passing filter (0.0 to 1.0).
    pub selectivity: f32,
    /// Number of samples evaluated.
    pub sample_size: usize,
    /// Number of samples that passed the filter.
    pub passed: usize,
}

/// Calculate oversample factor from selectivity.
///
/// Formula: `oversample = 1 / selectivity`, capped at `MAX_OVERSAMPLE`.
///
/// # Arguments
///
/// * `selectivity` - Estimated fraction of vectors passing (0.0 to 1.0)
///
/// # Returns
///
/// Oversample factor, always >= 1.0 and <= `MAX_OVERSAMPLE`.
///
/// # Example
///
/// ```rust
/// use edgevec::filter::strategy::{calculate_oversample, MAX_OVERSAMPLE};
///
/// assert_eq!(calculate_oversample(1.0), 1.0);
/// assert_eq!(calculate_oversample(0.5), 2.0);
/// assert_eq!(calculate_oversample(0.1), 10.0);
/// assert_eq!(calculate_oversample(0.01), MAX_OVERSAMPLE); // Capped
/// ```
#[must_use]
pub fn calculate_oversample(selectivity: f32) -> f32 {
    if selectivity <= 0.0 {
        return MAX_OVERSAMPLE;
    }
    (1.0 / selectivity).min(MAX_OVERSAMPLE)
}

/// Select strategy based on estimated selectivity.
///
/// Decision matrix:
/// - `selectivity > 0.8`: PreFilter (scan all metadata first)
/// - `selectivity < 0.05`: PostFilter with high oversample
/// - Otherwise: Hybrid with adaptive oversample
///
/// # Arguments
///
/// * `selectivity` - Estimated fraction of vectors passing (0.0 to 1.0)
///
/// # Returns
///
/// The recommended `FilterStrategy` for the given selectivity.
///
/// # Example
///
/// ```rust
/// use edgevec::filter::strategy::{select_strategy, FilterStrategy};
///
/// // High selectivity -> PreFilter
/// assert_eq!(select_strategy(0.9), FilterStrategy::PreFilter);
///
/// // Low selectivity -> PostFilter
/// assert!(matches!(select_strategy(0.03), FilterStrategy::PostFilter { .. }));
///
/// // Medium selectivity -> Hybrid
/// assert!(matches!(select_strategy(0.3), FilterStrategy::Hybrid { .. }));
/// ```
#[must_use]
pub fn select_strategy(selectivity: f32) -> FilterStrategy {
    if selectivity > PREFILTER_THRESHOLD {
        FilterStrategy::PreFilter
    } else if selectivity < POSTFILTER_THRESHOLD {
        FilterStrategy::PostFilter {
            oversample: calculate_oversample(selectivity),
        }
    } else {
        FilterStrategy::Hybrid {
            oversample_min: 1.5,
            oversample_max: calculate_oversample(selectivity),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// METADATA STORE TRAIT
// ═══════════════════════════════════════════════════════════════════════════════

/// Trait for accessing vector metadata during selectivity estimation.
///
/// This trait abstracts metadata storage to allow selectivity estimation
/// without coupling to a specific storage implementation.
///
/// # Example
///
/// ```rust,ignore
/// use edgevec::filter::strategy::MetadataStore;
///
/// struct MyMetadataStore {
///     metadata: Vec<HashMap<String, MetadataValue>>,
/// }
///
/// impl MetadataStore for MyMetadataStore {
///     fn get_metadata(&self, id: usize) -> Option<&HashMap<String, MetadataValue>> {
///         self.metadata.get(id)
///     }
///
///     fn len(&self) -> usize {
///         self.metadata.len()
///     }
/// }
/// ```
pub trait MetadataStore {
    /// Get metadata for a vector by its ID.
    ///
    /// Returns `None` if the ID is invalid or the vector has no metadata.
    fn get_metadata(&self, id: usize) -> Option<&HashMap<String, MetadataValue>>;

    /// Get the total number of vectors in the store.
    fn len(&self) -> usize;

    /// Check if the store is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Estimate filter selectivity by sampling random vectors.
///
/// Samples up to `SELECTIVITY_SAMPLE_SIZE` vectors, evaluates the filter on
/// each, and returns the fraction that pass.
///
/// # Arguments
///
/// * `filter` - The filter expression to evaluate
/// * `metadata_store` - Access to vector metadata
/// * `seed` - Optional RNG seed for reproducibility (uses entropy if None)
///
/// # Returns
///
/// `SelectivityEstimate` with selectivity clamped to \[0.01, 1.0\].
///
/// # Complexity
///
/// O(min(n, SELECTIVITY_SAMPLE_SIZE) × filter_complexity)
///
/// # Example
///
/// ```rust,ignore
/// use edgevec::filter::strategy::estimate_selectivity;
/// use edgevec::filter::parse;
///
/// let filter = parse("category = \"gpu\"").unwrap();
/// let estimate = estimate_selectivity(&filter, &metadata_store, Some(42));
///
/// println!("Estimated selectivity: {:.2}%", estimate.selectivity * 100.0);
/// ```
pub fn estimate_selectivity<M: MetadataStore>(
    filter: &FilterExpr,
    metadata_store: &M,
    seed: Option<u64>,
) -> SelectivityEstimate {
    let total_vectors = metadata_store.len();

    // Handle empty store
    if total_vectors == 0 {
        return SelectivityEstimate {
            selectivity: 0.0,
            sample_size: 0,
            passed: 0,
        };
    }

    // Determine sample size (don't sample more than we have)
    let sample_size = SELECTIVITY_SAMPLE_SIZE.min(total_vectors);

    // Generate random sample indices using deterministic RNG
    let mut rng = match seed {
        Some(s) => ChaCha8Rng::seed_from_u64(s),
        None => ChaCha8Rng::from_entropy(),
    };

    let mut indices: Vec<usize> = (0..total_vectors).collect();
    indices.shuffle(&mut rng);
    let sample_indices = &indices[..sample_size];

    // Evaluate filter on each sample
    let mut passed = 0;
    for &idx in sample_indices {
        if let Some(metadata) = metadata_store.get_metadata(idx) {
            // Treat errors as non-matching (conservative approach)
            if evaluate(filter, metadata).unwrap_or(false) {
                passed += 1;
            }
        }
    }

    // Calculate selectivity, clamped to avoid division issues
    // Note: sample_size ≤ 100 (SELECTIVITY_SAMPLE_SIZE), so precision loss is acceptable
    #[allow(clippy::cast_precision_loss)]
    let raw_selectivity = (passed as f32) / (sample_size as f32);
    // Clamp to [0.01, 1.0] to avoid zero selectivity (infinite oversample)
    let selectivity = raw_selectivity.clamp(0.01, 1.0);

    SelectivityEstimate {
        selectivity,
        sample_size,
        passed,
    }
}

/// Helper to create SelectivityEstimate directly for testing/manual use.
impl SelectivityEstimate {
    /// Create a new selectivity estimate.
    #[must_use]
    pub fn new(selectivity: f32, sample_size: usize, passed: usize) -> Self {
        Self {
            selectivity: selectivity.clamp(0.0, 1.0),
            sample_size,
            passed,
        }
    }

    /// Create an estimate indicating zero selectivity (no matches).
    #[must_use]
    pub fn zero() -> Self {
        Self {
            selectivity: 0.01, // Clamped minimum
            sample_size: 0,
            passed: 0,
        }
    }

    /// Create an estimate indicating full selectivity (all match).
    #[must_use]
    pub fn full() -> Self {
        Self {
            selectivity: 1.0,
            sample_size: 0,
            passed: 0,
        }
    }

    /// Get the confidence level based on sample size.
    ///
    /// Larger samples provide higher confidence.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // sample_size is typically ≤ 100
    pub fn confidence(&self) -> f32 {
        if self.sample_size == 0 {
            return 0.0;
        }
        // Simple confidence metric: min(sample_size / 100, 1.0)
        (self.sample_size as f32 / 100.0).min(1.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// EDGE CASE DETECTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Detect tautological filters (always true).
///
/// Tautologies can be detected statically and allow bypassing the filter
/// entirely during search.
///
/// # Detected Patterns
///
/// - `true` literal
/// - `a OR NOT a`
/// - Nested tautologies in OR expressions
///
/// # Example
///
/// ```rust
/// use edgevec::filter::strategy::is_tautology;
/// use edgevec::filter::FilterExpr;
///
/// // TRUE literal is tautology
/// assert!(is_tautology(&FilterExpr::LiteralBool(true)));
///
/// // a OR NOT a is tautology
/// let a = FilterExpr::Field("x".to_string());
/// let not_a = FilterExpr::Not(Box::new(a.clone()));
/// let or = FilterExpr::Or(Box::new(a), Box::new(not_a));
/// assert!(is_tautology(&or));
/// ```
#[must_use]
pub fn is_tautology(filter: &FilterExpr) -> bool {
    match filter {
        FilterExpr::LiteralBool(true) => true,
        FilterExpr::Or(left, right) => {
            // Check for a OR NOT a pattern
            if are_complementary(left, right) {
                return true;
            }
            // Recurse: if either branch is tautology, entire OR is tautology
            is_tautology(left) || is_tautology(right)
        }
        FilterExpr::And(left, right) => {
            // Both sides must be tautologies for AND to be tautology
            is_tautology(left) && is_tautology(right)
        }
        FilterExpr::Not(inner) => is_contradiction(inner),
        _ => false,
    }
}

/// Detect contradictory filters (always false).
///
/// Contradictions can be detected statically and allow returning empty
/// results immediately without searching.
///
/// # Detected Patterns
///
/// - `false` literal
/// - `a AND NOT a`
/// - Impossible ranges (e.g., `x > 10 AND x < 5`)
/// - Nested contradictions
///
/// # Example
///
/// ```rust
/// use edgevec::filter::strategy::is_contradiction;
/// use edgevec::filter::FilterExpr;
///
/// // FALSE literal is contradiction
/// assert!(is_contradiction(&FilterExpr::LiteralBool(false)));
///
/// // a AND NOT a is contradiction
/// let a = FilterExpr::Field("x".to_string());
/// let not_a = FilterExpr::Not(Box::new(a.clone()));
/// let and = FilterExpr::And(Box::new(a), Box::new(not_a));
/// assert!(is_contradiction(&and));
/// ```
#[must_use]
pub fn is_contradiction(filter: &FilterExpr) -> bool {
    match filter {
        FilterExpr::LiteralBool(false) => true,
        FilterExpr::And(left, right) => {
            // Check for a AND NOT a pattern
            if are_complementary(left, right) {
                return true;
            }
            // Check for impossible ranges
            if is_impossible_range(left, right) || is_impossible_range(right, left) {
                return true;
            }
            // Recurse: if either branch is contradiction, entire AND is contradiction
            is_contradiction(left) || is_contradiction(right)
        }
        FilterExpr::Or(left, right) => {
            // Both sides must be contradictions for OR to be contradiction
            is_contradiction(left) && is_contradiction(right)
        }
        FilterExpr::Not(inner) => is_tautology(inner),
        _ => false,
    }
}

/// Check if two expressions are complements (one is NOT of the other).
fn are_complementary(a: &FilterExpr, b: &FilterExpr) -> bool {
    match (a, b) {
        (FilterExpr::Not(inner), other) | (other, FilterExpr::Not(inner)) => {
            inner.as_ref() == other
        }
        _ => false,
    }
}

/// Check if two range conditions are impossible together.
///
/// Detects patterns like `x > 10 AND x < 5` which can never be true.
fn is_impossible_range(a: &FilterExpr, b: &FilterExpr) -> bool {
    // Check a > high AND a < low patterns
    match (a, b) {
        // x > v1 AND x < v2 where v1 >= v2
        (
            FilterExpr::Gt(field1, val1) | FilterExpr::Ge(field1, val1),
            FilterExpr::Lt(field2, val2) | FilterExpr::Le(field2, val2),
        ) => {
            if field1 != field2 {
                return false;
            }
            compare_values_gte(val1, val2)
        }
        // x < v1 AND x > v2 where v1 <= v2
        (
            FilterExpr::Lt(field1, val1) | FilterExpr::Le(field1, val1),
            FilterExpr::Gt(field2, val2) | FilterExpr::Ge(field2, val2),
        ) => {
            if field1 != field2 {
                return false;
            }
            compare_values_gte(val2, val1)
        }
        _ => false,
    }
}

/// Compare two literal values, returning true if left >= right.
#[allow(clippy::cast_precision_loss)]
fn compare_values_gte(left: &FilterExpr, right: &FilterExpr) -> bool {
    match (left, right) {
        (FilterExpr::LiteralInt(v1), FilterExpr::LiteralInt(v2)) => v1 >= v2,
        (FilterExpr::LiteralFloat(v1), FilterExpr::LiteralFloat(v2)) => v1 >= v2,
        (FilterExpr::LiteralInt(v1), FilterExpr::LiteralFloat(v2)) => (*v1 as f64) >= *v2,
        (FilterExpr::LiteralFloat(v1), FilterExpr::LiteralInt(v2)) => *v1 >= (*v2 as f64),
        _ => false,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
#[allow(clippy::float_cmp)] // Tests use exact float comparisons for deterministic values
mod tests {
    use super::*;
    use crate::filter::parse;

    // ═══════════════════════════════════════════════════════════════════════════
    // FILTER STRATEGY TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_default_is_auto() {
        assert_eq!(FilterStrategy::default(), FilterStrategy::Auto);
    }

    #[test]
    fn test_validate_post_filter_valid() {
        assert!(FilterStrategy::PostFilter { oversample: 1.0 }
            .validate()
            .is_ok());
        assert!(FilterStrategy::PostFilter { oversample: 5.0 }
            .validate()
            .is_ok());
        assert!(FilterStrategy::PostFilter { oversample: 10.0 }
            .validate()
            .is_ok());
    }

    #[test]
    fn test_validate_post_filter_invalid() {
        assert!(FilterStrategy::PostFilter { oversample: 0.5 }
            .validate()
            .is_err());
        assert!(FilterStrategy::PostFilter { oversample: 15.0 }
            .validate()
            .is_err());
        assert!(FilterStrategy::PostFilter { oversample: 0.0 }
            .validate()
            .is_err());
        assert!(FilterStrategy::PostFilter { oversample: -1.0 }
            .validate()
            .is_err());
    }

    #[test]
    fn test_validate_hybrid_valid() {
        assert!(FilterStrategy::Hybrid {
            oversample_min: 1.0,
            oversample_max: 10.0
        }
        .validate()
        .is_ok());
        assert!(FilterStrategy::Hybrid {
            oversample_min: 1.5,
            oversample_max: 5.0
        }
        .validate()
        .is_ok());
        assert!(FilterStrategy::HYBRID_DEFAULT.validate().is_ok());
    }

    #[test]
    fn test_validate_hybrid_invalid() {
        // min < 1.0
        assert!(FilterStrategy::Hybrid {
            oversample_min: 0.5,
            oversample_max: 10.0
        }
        .validate()
        .is_err());
        // max < min
        assert!(FilterStrategy::Hybrid {
            oversample_min: 5.0,
            oversample_max: 3.0
        }
        .validate()
        .is_err());
        // max > MAX_OVERSAMPLE
        assert!(FilterStrategy::Hybrid {
            oversample_min: 1.0,
            oversample_max: 15.0
        }
        .validate()
        .is_err());
    }

    #[test]
    fn test_validate_prefilter_auto_always_valid() {
        assert!(FilterStrategy::PreFilter.validate().is_ok());
        assert!(FilterStrategy::Auto.validate().is_ok());
    }

    #[test]
    fn test_post_filter_default() {
        assert_eq!(
            FilterStrategy::POST_FILTER_DEFAULT,
            FilterStrategy::PostFilter { oversample: 3.0 }
        );
    }

    #[test]
    fn test_hybrid_default() {
        assert_eq!(
            FilterStrategy::HYBRID_DEFAULT,
            FilterStrategy::Hybrid {
                oversample_min: 1.5,
                oversample_max: 10.0
            }
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SELECTIVITY CALCULATION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_calculate_oversample_normal() {
        assert!((calculate_oversample(1.0) - 1.0).abs() < 0.001);
        assert!((calculate_oversample(0.5) - 2.0).abs() < 0.001);
        assert!((calculate_oversample(0.25) - 4.0).abs() < 0.001);
        assert!((calculate_oversample(0.1) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_oversample_capped() {
        assert_eq!(calculate_oversample(0.05), MAX_OVERSAMPLE);
        assert_eq!(calculate_oversample(0.01), MAX_OVERSAMPLE);
        assert_eq!(calculate_oversample(0.001), MAX_OVERSAMPLE);
    }

    #[test]
    fn test_calculate_oversample_edge_cases() {
        assert_eq!(calculate_oversample(0.0), MAX_OVERSAMPLE);
        assert_eq!(calculate_oversample(-0.1), MAX_OVERSAMPLE);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STRATEGY SELECTION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_select_strategy_high_selectivity() {
        assert_eq!(select_strategy(0.9), FilterStrategy::PreFilter);
        assert_eq!(select_strategy(0.85), FilterStrategy::PreFilter);
        assert_eq!(select_strategy(0.81), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_select_strategy_at_threshold() {
        // At 0.8 exactly, should NOT be PreFilter (threshold is >0.8)
        assert!(matches!(
            select_strategy(0.8),
            FilterStrategy::Hybrid { .. }
        ));
        // At 0.05 exactly, should be Hybrid (threshold is <0.05)
        assert!(matches!(
            select_strategy(0.05),
            FilterStrategy::Hybrid { .. }
        ));
    }

    #[test]
    fn test_select_strategy_low_selectivity() {
        match select_strategy(0.03) {
            FilterStrategy::PostFilter { oversample } => {
                assert!(oversample > 5.0);
                assert!(oversample <= MAX_OVERSAMPLE);
            }
            _ => panic!("Expected PostFilter"),
        }

        match select_strategy(0.01) {
            FilterStrategy::PostFilter { oversample } => {
                assert_eq!(oversample, MAX_OVERSAMPLE);
            }
            _ => panic!("Expected PostFilter"),
        }
    }

    #[test]
    fn test_select_strategy_medium_selectivity() {
        match select_strategy(0.3) {
            FilterStrategy::Hybrid {
                oversample_min,
                oversample_max,
            } => {
                assert!((oversample_min - 1.5).abs() < 0.001);
                assert!(oversample_max > 3.0);
            }
            _ => panic!("Expected Hybrid"),
        }

        match select_strategy(0.5) {
            FilterStrategy::Hybrid { .. } => {}
            _ => panic!("Expected Hybrid"),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TAUTOLOGY DETECTION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_is_tautology_true_literal() {
        assert!(is_tautology(&FilterExpr::LiteralBool(true)));
    }

    #[test]
    fn test_is_tautology_false_literal() {
        assert!(!is_tautology(&FilterExpr::LiteralBool(false)));
    }

    #[test]
    fn test_is_tautology_a_or_not_a() {
        let a = FilterExpr::Field("x".to_string());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let or = FilterExpr::Or(Box::new(a), Box::new(not_a));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_is_tautology_not_a_or_a() {
        let a = FilterExpr::Field("x".to_string());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let or = FilterExpr::Or(Box::new(not_a), Box::new(a));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_is_tautology_nested_or() {
        // (a AND b) OR TRUE
        let a = FilterExpr::Field("x".to_string());
        let b = FilterExpr::Field("y".to_string());
        let and = FilterExpr::And(Box::new(a), Box::new(b));
        let or = FilterExpr::Or(Box::new(and), Box::new(FilterExpr::LiteralBool(true)));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_is_tautology_simple_expression() {
        let filter = parse("x = 5").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_is_tautology_not_contradiction() {
        // NOT(false) is tautology
        let filter = FilterExpr::Not(Box::new(FilterExpr::LiteralBool(false)));
        assert!(is_tautology(&filter));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONTRADICTION DETECTION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_is_contradiction_false_literal() {
        assert!(is_contradiction(&FilterExpr::LiteralBool(false)));
    }

    #[test]
    fn test_is_contradiction_true_literal() {
        assert!(!is_contradiction(&FilterExpr::LiteralBool(true)));
    }

    #[test]
    fn test_is_contradiction_a_and_not_a() {
        let a = FilterExpr::Field("x".to_string());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let and = FilterExpr::And(Box::new(a), Box::new(not_a));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_is_contradiction_not_a_and_a() {
        let a = FilterExpr::Field("x".to_string());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let and = FilterExpr::And(Box::new(not_a), Box::new(a));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_is_contradiction_impossible_range_gt_lt() {
        // x > 10 AND x < 5 is impossible
        let gt10 = parse("x > 10").unwrap();
        let lt5 = parse("x < 5").unwrap();
        let and = FilterExpr::And(Box::new(gt10), Box::new(lt5));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_is_contradiction_impossible_range_ge_le() {
        // x >= 10 AND x <= 5 is impossible
        let ge10 = parse("x >= 10").unwrap();
        let le5 = parse("x <= 5").unwrap();
        let and = FilterExpr::And(Box::new(ge10), Box::new(le5));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_is_contradiction_impossible_range_reverse() {
        // x < 5 AND x > 10 is impossible (order reversed)
        let lt5 = parse("x < 5").unwrap();
        let gt10 = parse("x > 10").unwrap();
        let and = FilterExpr::And(Box::new(lt5), Box::new(gt10));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_is_contradiction_possible_range() {
        // x > 5 AND x < 10 is possible
        let gt5 = parse("x > 5").unwrap();
        let lt10 = parse("x < 10").unwrap();
        let and = FilterExpr::And(Box::new(gt5), Box::new(lt10));
        assert!(!is_contradiction(&and));
    }

    #[test]
    fn test_is_contradiction_float_range() {
        // x > 10.5 AND x < 5.0 is impossible
        let gt = parse("x > 10.5").unwrap();
        let lt = parse("x < 5.0").unwrap();
        let and = FilterExpr::And(Box::new(gt), Box::new(lt));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_is_contradiction_nested_or() {
        // FALSE OR FALSE is contradiction
        let or = FilterExpr::Or(
            Box::new(FilterExpr::LiteralBool(false)),
            Box::new(FilterExpr::LiteralBool(false)),
        );
        assert!(is_contradiction(&or));
    }

    #[test]
    fn test_is_contradiction_simple_expression() {
        let filter = parse("x = 5").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_is_contradiction_not_tautology() {
        // NOT(true) is contradiction
        let filter = FilterExpr::Not(Box::new(FilterExpr::LiteralBool(true)));
        assert!(is_contradiction(&filter));
    }

    #[test]
    fn test_is_contradiction_different_fields() {
        // x > 10 AND y < 5 is possible (different fields)
        let gt = parse("x > 10").unwrap();
        let lt = parse("y < 5").unwrap();
        let and = FilterExpr::And(Box::new(gt), Box::new(lt));
        assert!(!is_contradiction(&and));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSTANTS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_constants_have_expected_values() {
        assert_eq!(MAX_OVERSAMPLE, 10.0);
        assert_eq!(DEFAULT_OVERSAMPLE, 3.0);
        assert_eq!(EF_CAP, 1000);
        assert_eq!(SELECTIVITY_SAMPLE_SIZE, 100);
        assert_eq!(PREFILTER_THRESHOLD, 0.8);
        assert_eq!(POSTFILTER_THRESHOLD, 0.05);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SELECTIVITY ESTIMATION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Mock metadata store for testing.
    struct MockMetadataStore {
        metadata: Vec<HashMap<String, MetadataValue>>,
    }

    impl MockMetadataStore {
        #[allow(dead_code)]
        fn new(metadata: Vec<HashMap<String, MetadataValue>>) -> Self {
            Self { metadata }
        }

        fn empty() -> Self {
            Self { metadata: vec![] }
        }

        fn all_active(count: usize) -> Self {
            let metadata = (0..count)
                .map(|_| {
                    let mut m = HashMap::new();
                    m.insert("active".to_string(), MetadataValue::Boolean(true));
                    m
                })
                .collect();
            Self { metadata }
        }

        fn half_active(count: usize) -> Self {
            let metadata = (0..count)
                .map(|i| {
                    let mut m = HashMap::new();
                    m.insert("active".to_string(), MetadataValue::Boolean(i % 2 == 0));
                    m
                })
                .collect();
            Self { metadata }
        }

        fn none_active(count: usize) -> Self {
            let metadata = (0..count)
                .map(|_| {
                    let mut m = HashMap::new();
                    m.insert("active".to_string(), MetadataValue::Boolean(false));
                    m
                })
                .collect();
            Self { metadata }
        }
    }

    impl MetadataStore for MockMetadataStore {
        fn get_metadata(&self, id: usize) -> Option<&HashMap<String, MetadataValue>> {
            self.metadata.get(id)
        }

        fn len(&self) -> usize {
            self.metadata.len()
        }
    }

    #[test]
    fn test_estimate_selectivity_empty_store() {
        let filter = parse("active = true").unwrap();
        let store = MockMetadataStore::empty();
        let estimate = estimate_selectivity(&filter, &store, Some(42));

        assert_eq!(estimate.selectivity, 0.0);
        assert_eq!(estimate.sample_size, 0);
        assert_eq!(estimate.passed, 0);
    }

    #[test]
    fn test_estimate_selectivity_all_pass() {
        let filter = parse("active = true").unwrap();
        let store = MockMetadataStore::all_active(200);
        let estimate = estimate_selectivity(&filter, &store, Some(42));

        assert_eq!(estimate.selectivity, 1.0);
        assert_eq!(estimate.sample_size, SELECTIVITY_SAMPLE_SIZE);
        assert_eq!(estimate.passed, SELECTIVITY_SAMPLE_SIZE);
    }

    #[test]
    fn test_estimate_selectivity_none_pass() {
        let filter = parse("active = true").unwrap();
        let store = MockMetadataStore::none_active(200);
        let estimate = estimate_selectivity(&filter, &store, Some(42));

        // Selectivity is clamped to 0.01 minimum
        assert_eq!(estimate.selectivity, 0.01);
        assert_eq!(estimate.sample_size, SELECTIVITY_SAMPLE_SIZE);
        assert_eq!(estimate.passed, 0);
    }

    #[test]
    fn test_estimate_selectivity_half_pass() {
        let filter = parse("active = true").unwrap();
        let store = MockMetadataStore::half_active(200);
        let estimate = estimate_selectivity(&filter, &store, Some(42));

        // With alternating true/false and random sampling, expect ~50%
        assert!(estimate.selectivity > 0.3 && estimate.selectivity < 0.7);
        assert_eq!(estimate.sample_size, SELECTIVITY_SAMPLE_SIZE);
    }

    #[test]
    fn test_estimate_selectivity_small_store() {
        let filter = parse("active = true").unwrap();
        let store = MockMetadataStore::all_active(10); // Less than SELECTIVITY_SAMPLE_SIZE
        let estimate = estimate_selectivity(&filter, &store, Some(42));

        assert_eq!(estimate.selectivity, 1.0);
        assert_eq!(estimate.sample_size, 10); // Sample size = store size
        assert_eq!(estimate.passed, 10);
    }

    #[test]
    fn test_estimate_selectivity_deterministic() {
        let filter = parse("active = true").unwrap();
        let store = MockMetadataStore::half_active(200);

        // Same seed should produce same result
        let estimate1 = estimate_selectivity(&filter, &store, Some(42));
        let estimate2 = estimate_selectivity(&filter, &store, Some(42));

        assert_eq!(estimate1.selectivity, estimate2.selectivity);
        assert_eq!(estimate1.passed, estimate2.passed);
    }

    #[test]
    fn test_estimate_selectivity_different_seeds() {
        let filter = parse("active = true").unwrap();
        let store = MockMetadataStore::half_active(1000);

        // Different seeds may produce different samples
        let estimate1 = estimate_selectivity(&filter, &store, Some(1));
        let estimate2 = estimate_selectivity(&filter, &store, Some(999));

        // Both should be around 50%, but exact values may differ
        assert!(estimate1.selectivity > 0.3 && estimate1.selectivity < 0.7);
        assert!(estimate2.selectivity > 0.3 && estimate2.selectivity < 0.7);
    }

    #[test]
    fn test_selectivity_estimate_new() {
        let estimate = SelectivityEstimate::new(0.5, 100, 50);
        assert_eq!(estimate.selectivity, 0.5);
        assert_eq!(estimate.sample_size, 100);
        assert_eq!(estimate.passed, 50);
    }

    #[test]
    fn test_selectivity_estimate_clamping() {
        // Selectivity clamped to [0.0, 1.0]
        let estimate = SelectivityEstimate::new(1.5, 100, 150);
        assert_eq!(estimate.selectivity, 1.0);

        let estimate = SelectivityEstimate::new(-0.5, 100, 0);
        assert_eq!(estimate.selectivity, 0.0);
    }

    #[test]
    fn test_selectivity_estimate_zero() {
        let estimate = SelectivityEstimate::zero();
        assert_eq!(estimate.selectivity, 0.01); // Clamped minimum
        assert_eq!(estimate.sample_size, 0);
        assert_eq!(estimate.passed, 0);
    }

    #[test]
    fn test_selectivity_estimate_full() {
        let estimate = SelectivityEstimate::full();
        assert_eq!(estimate.selectivity, 1.0);
    }

    #[test]
    fn test_selectivity_estimate_confidence() {
        let estimate = SelectivityEstimate::new(0.5, 100, 50);
        assert_eq!(estimate.confidence(), 1.0);

        let estimate = SelectivityEstimate::new(0.5, 50, 25);
        assert_eq!(estimate.confidence(), 0.5);

        let estimate = SelectivityEstimate::new(0.5, 0, 0);
        assert_eq!(estimate.confidence(), 0.0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PROPERTY TESTS FOR STRATEGY MODULE
    // ═══════════════════════════════════════════════════════════════════════════

    use proptest::prelude::*;

    proptest! {
        /// Property: Selectivity is always clamped to [0.0, 1.0]
        ///
        /// No matter what input value, the resulting selectivity should be in valid range.
        #[test]
        fn prop_selectivity_always_clamped(raw_selectivity in -100.0f32..100.0f32) {
            let estimate = SelectivityEstimate::new(raw_selectivity, 100, 50);
            prop_assert!(estimate.selectivity >= 0.0, "Selectivity should be >= 0.0");
            prop_assert!(estimate.selectivity <= 1.0, "Selectivity should be <= 1.0");
        }

        /// Property: Oversample is always bounded [1.0, MAX_OVERSAMPLE] for valid selectivity
        ///
        /// calculate_oversample should return values within valid bounds.
        /// Note: Selectivity should be in [0.0, 1.0] per domain rules.
        #[test]
        fn prop_oversample_always_bounded(selectivity in 0.0f32..=1.0f32) {
            let oversample = calculate_oversample(selectivity);
            prop_assert!(oversample >= 1.0, "Oversample should be >= 1.0, got {} for selectivity {}", oversample, selectivity);
            prop_assert!(oversample <= MAX_OVERSAMPLE, "Oversample should be <= MAX_OVERSAMPLE, got {} for selectivity {}", oversample, selectivity);
        }

        /// Property: Tautology implies not contradiction
        ///
        /// A filter cannot be both a tautology and a contradiction.
        #[test]
        fn prop_tautology_implies_not_contradiction(b in proptest::bool::ANY) {
            let expr = FilterExpr::LiteralBool(b);
            if is_tautology(&expr) {
                prop_assert!(!is_contradiction(&expr), "Tautology cannot be contradiction");
            }
        }

        /// Property: Contradiction implies not tautology
        ///
        /// A filter cannot be both a contradiction and a tautology.
        #[test]
        fn prop_contradiction_implies_not_tautology(b in proptest::bool::ANY) {
            let expr = FilterExpr::LiteralBool(b);
            if is_contradiction(&expr) {
                prop_assert!(!is_tautology(&expr), "Contradiction cannot be tautology");
            }
        }

        /// Property: select_strategy returns deterministic results
        ///
        /// Same input selectivity should always produce same strategy type.
        #[test]
        fn prop_strategy_selection_deterministic(selectivity in 0.0f32..1.0f32) {
            let strategy1 = select_strategy(selectivity);
            let strategy2 = select_strategy(selectivity);
            prop_assert_eq!(strategy1, strategy2, "Strategy selection should be deterministic");
        }
    }
}

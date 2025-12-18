# Week 23 Day 3: Strategy & Integration

**Date:** Day 3 of Week 23
**Focus:** Filtering Strategy Selection & HNSW Integration
**Agent:** RUST_ENGINEER
**Total Hours:** 12h
**Status:** [PLANNED]

---

## Executive Summary

Day 3 implements the strategy selection system that determines how filtering integrates with HNSW search. This is the bridge between filter evaluation (Day 2) and the actual search API.

**Prerequisites:**
- W23.2.1 (core evaluate function) COMPLETE
- W23.2.2-W23.2.4 (all operators) COMPLETE
- `src/filter/evaluator.rs` fully functional

---

## Tasks Overview

| Task ID | Description | Hours | Priority |
|:--------|:------------|:------|:---------|
| W23.3.1 | FilterStrategy enum and configuration | 2h | P0 |
| W23.3.2 | Selectivity estimation via sampling | 3h | P0 |
| W23.3.3 | search_filtered() public API | 4h | P0 |
| W23.3.4 | Edge case handlers (contradiction, tautology) | 3h | P0 |

---

## Task W23.3.1: FilterStrategy Enum and Configuration

### Description
Define the FilterStrategy enum with all variants and configuration constants.

### Hours: 2h

### Specification

**File:** `src/filter/strategy.rs`

```rust
//! Filter strategy selection for EdgeVec.
//!
//! Determines how filtering integrates with HNSW search based on
//! estimated selectivity and configured parameters.

/// Maximum oversample factor to prevent ef explosion.
pub const MAX_OVERSAMPLE: f32 = 10.0;

/// Default oversample when selectivity is unknown.
pub const DEFAULT_OVERSAMPLE: f32 = 3.0;

/// Absolute cap on ef_search to bound latency.
pub const EF_CAP: usize = 1000;

/// Minimum sample size for selectivity estimation.
pub const SELECTIVITY_SAMPLE_SIZE: usize = 100;

/// Selectivity threshold above which pre-filter is preferred.
pub const PREFILTER_THRESHOLD: f32 = 0.8;

/// Selectivity threshold below which post-filter is sufficient.
pub const POSTFILTER_THRESHOLD: f32 = 0.05;

/// Strategy for combining filtering with HNSW search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterStrategy {
    /// Post-filter with fixed oversample factor.
    PostFilter {
        /// Oversample factor (1.0 = no oversampling).
        oversample: f32,
    },

    /// Pre-filter (full metadata scan, then search on subset).
    PreFilter,

    /// Hybrid with bounded oversample (adaptive based on selectivity).
    Hybrid {
        /// Minimum oversample (floor).
        oversample_min: f32,
        /// Maximum oversample (ceiling).
        oversample_max: f32,
    },

    /// Automatic strategy selection based on estimated selectivity.
    Auto,
}

impl Default for FilterStrategy {
    fn default() -> Self {
        FilterStrategy::Auto
    }
}

impl FilterStrategy {
    /// Post-filter with default oversample (3x).
    pub const POST_FILTER_DEFAULT: Self = FilterStrategy::PostFilter { oversample: 3.0 };

    /// Hybrid with default bounds (1.5x to 10x).
    pub const HYBRID_DEFAULT: Self = FilterStrategy::Hybrid {
        oversample_min: 1.5,
        oversample_max: 10.0,
    };

    /// Validate strategy configuration.
    pub fn validate(&self) -> Result<(), FilterError> {
        match self {
            FilterStrategy::PostFilter { oversample } => {
                if *oversample < 1.0 {
                    return Err(FilterError::InvalidStrategy(
                        "oversample must be >= 1.0".into(),
                    ));
                }
                if *oversample > MAX_OVERSAMPLE {
                    return Err(FilterError::InvalidStrategy(
                        format!("oversample must be <= {}", MAX_OVERSAMPLE),
                    ));
                }
                Ok(())
            }
            FilterStrategy::Hybrid { oversample_min, oversample_max } => {
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
                    return Err(FilterError::InvalidStrategy(
                        format!("oversample_max must be <= {}", MAX_OVERSAMPLE),
                    ));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
```

### Acceptance Criteria
- [ ] FilterStrategy enum has all 4 variants: PostFilter, PreFilter, Hybrid, Auto
- [ ] All constants documented with rationale
- [ ] `validate()` catches invalid configurations
- [ ] Default is Auto
- [ ] `cargo test filter::strategy::tests` passes

### Test Cases
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_auto() {
        assert_eq!(FilterStrategy::default(), FilterStrategy::Auto);
    }

    #[test]
    fn test_validate_post_filter() {
        assert!(FilterStrategy::PostFilter { oversample: 1.0 }.validate().is_ok());
        assert!(FilterStrategy::PostFilter { oversample: 10.0 }.validate().is_ok());
        assert!(FilterStrategy::PostFilter { oversample: 0.5 }.validate().is_err());
        assert!(FilterStrategy::PostFilter { oversample: 15.0 }.validate().is_err());
    }

    #[test]
    fn test_validate_hybrid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 1.5, oversample_max: 10.0 }.validate().is_ok());
        assert!(FilterStrategy::Hybrid { oversample_min: 0.5, oversample_max: 10.0 }.validate().is_err());
        assert!(FilterStrategy::Hybrid { oversample_min: 5.0, oversample_max: 3.0 }.validate().is_err());
    }
}
```

---

## Task W23.3.2: Selectivity Estimation via Sampling

### Description
Implement selectivity estimation by sampling random vectors and evaluating the filter.

### Hours: 3h

### Specification

**File:** `src/filter/strategy.rs` (continued)

```rust
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Selectivity estimation result.
#[derive(Debug, Clone, Copy)]
pub struct SelectivityEstimate {
    /// Estimated fraction of vectors passing filter (0.0 to 1.0).
    pub selectivity: f32,
    /// Number of samples evaluated.
    pub sample_size: usize,
    /// Number of samples that passed the filter.
    pub passed: usize,
}

/// Estimate selectivity by sampling random vectors.
///
/// # Arguments
/// * `filter` - The filter expression to evaluate
/// * `metadata_store` - Access to vector metadata
/// * `total_vectors` - Total number of vectors in index
///
/// # Returns
/// SelectivityEstimate with selectivity clamped to [0.01, 1.0]
///
/// # Complexity
/// O(SELECTIVITY_SAMPLE_SIZE × filter_complexity)
pub fn estimate_selectivity<M: MetadataStore>(
    filter: &FilterExpr,
    metadata_store: &M,
    total_vectors: usize,
) -> SelectivityEstimate {
    if total_vectors == 0 {
        return SelectivityEstimate {
            selectivity: 0.0,
            sample_size: 0,
            passed: 0,
        };
    }

    // Determine sample size (don't sample more than we have)
    let sample_size = SELECTIVITY_SAMPLE_SIZE.min(total_vectors);

    // Generate random sample indices
    let mut rng = thread_rng();
    let mut indices: Vec<usize> = (0..total_vectors).collect();
    indices.shuffle(&mut rng);
    let sample_indices = &indices[..sample_size];

    // Evaluate filter on each sample
    let mut passed = 0;
    for &idx in sample_indices {
        if let Some(metadata) = metadata_store.get(idx) {
            match evaluate(filter, &metadata) {
                Ok(true) => passed += 1,
                Ok(false) => {}
                Err(_) => {} // Treat errors as non-matching
            }
        }
    }

    // Calculate selectivity, clamped to avoid division issues
    let raw_selectivity = (passed as f32) / (sample_size as f32);
    let selectivity = raw_selectivity.max(0.01).min(1.0);

    SelectivityEstimate {
        selectivity,
        sample_size,
        passed,
    }
}

/// Calculate oversample factor from selectivity.
///
/// oversample = 1 / selectivity, capped at MAX_OVERSAMPLE
pub fn calculate_oversample(selectivity: f32) -> f32 {
    (1.0 / selectivity).min(MAX_OVERSAMPLE)
}

/// Select strategy based on estimated selectivity.
///
/// Decision matrix:
/// - selectivity > PREFILTER_THRESHOLD (80%): PreFilter
/// - selectivity < POSTFILTER_THRESHOLD (5%): PostFilter with high oversample
/// - otherwise: Hybrid with adaptive oversample
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
```

### Acceptance Criteria
- [ ] `estimate_selectivity()` samples correctly
- [ ] Empty index returns 0.0 selectivity
- [ ] Selectivity clamped to [0.01, 1.0]
- [ ] `calculate_oversample()` respects MAX_OVERSAMPLE
- [ ] `select_strategy()` follows decision matrix
- [ ] `cargo test filter::strategy::selectivity` passes

### Test Cases
```rust
#[test]
fn test_estimate_selectivity_empty_index() {
    let filter = parse("x = 1").unwrap();
    let store = MockMetadataStore::empty();
    let estimate = estimate_selectivity(&filter, &store, 0);
    assert_eq!(estimate.selectivity, 0.0);
    assert_eq!(estimate.sample_size, 0);
}

#[test]
fn test_estimate_selectivity_all_pass() {
    let filter = parse("active = true").unwrap();
    let store = MockMetadataStore::all_active(1000);
    let estimate = estimate_selectivity(&filter, &store, 1000);
    assert!(estimate.selectivity > 0.95);
}

#[test]
fn test_calculate_oversample_bounds() {
    assert_eq!(calculate_oversample(1.0), 1.0);
    assert_eq!(calculate_oversample(0.1), 10.0);
    assert_eq!(calculate_oversample(0.01), 10.0); // Capped at MAX_OVERSAMPLE
}

#[test]
fn test_select_strategy_high_selectivity() {
    assert_eq!(select_strategy(0.9), FilterStrategy::PreFilter);
}

#[test]
fn test_select_strategy_low_selectivity() {
    match select_strategy(0.03) {
        FilterStrategy::PostFilter { oversample } => assert!(oversample > 5.0),
        _ => panic!("Expected PostFilter"),
    }
}

#[test]
fn test_select_strategy_medium_selectivity() {
    match select_strategy(0.3) {
        FilterStrategy::Hybrid { .. } => {}
        _ => panic!("Expected Hybrid"),
    }
}
```

---

## Task W23.3.3: search_filtered() Public API

### Description
Implement the main `search_filtered()` method that integrates filtering with HNSW search.

### Hours: 4h

### Specification

**File:** `src/hnsw/mod.rs` (add to HnswIndex impl)

```rust
/// Result from a filtered search operation.
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

impl HnswIndex {
    /// Search with optional filter and strategy.
    ///
    /// # Arguments
    /// * `query` - Query vector (same dimensions as indexed vectors)
    /// * `k` - Number of results to return
    /// * `filter` - Optional filter expression
    /// * `strategy` - Filter strategy (default: Auto)
    ///
    /// # Returns
    /// * `Ok(FilteredSearchResult)` - Search results with diagnostics
    /// * `Err(FilterError)` - On invalid filter or search failure
    ///
    /// # Performance
    /// - Auto strategy: <10ms P99 for 100k vectors
    /// - Pre-filter: O(n) + O(log m) where m = filtered count
    /// - Post-filter: O(log n × ef_search)
    ///
    /// # Example
    /// ```rust
    /// let filter = parse("category = \"gpu\" AND price < 500")?;
    /// let results = index.search_filtered(
    ///     &query,
    ///     10,
    ///     Some(&filter),
    ///     FilterStrategy::Auto,
    /// )?;
    /// ```
    pub fn search_filtered(
        &self,
        query: &[f32],
        k: usize,
        filter: Option<&FilterExpr>,
        strategy: FilterStrategy,
    ) -> Result<FilteredSearchResult, FilterError> {
        // Validate inputs
        strategy.validate()?;

        // No filter = standard search
        let filter = match filter {
            None => {
                let results = self.search(query, k)?;
                return Ok(FilteredSearchResult {
                    results,
                    complete: true,
                    observed_selectivity: 1.0,
                    strategy_used: strategy,
                    vectors_evaluated: k,
                });
            }
            Some(f) => f,
        };

        // Handle edge cases (tautology, contradiction)
        if let Some(result) = self.handle_filter_edge_cases(filter, k)? {
            return Ok(result);
        }

        // Determine actual strategy
        let actual_strategy = match strategy {
            FilterStrategy::Auto => {
                let estimate = estimate_selectivity(filter, &self.metadata, self.len());
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
            FilterStrategy::Hybrid { oversample_min, oversample_max } => {
                self.search_hybrid(query, k, filter, oversample_min, oversample_max)
            }
            FilterStrategy::Auto => unreachable!(), // Already resolved above
        }
    }

    /// Pre-filter strategy: scan all metadata, then search on passing subset.
    fn search_prefilter(
        &self,
        query: &[f32],
        k: usize,
        filter: &FilterExpr,
    ) -> Result<FilteredSearchResult, FilterError> {
        // Build bitset of passing vectors
        let mut passing_ids = Vec::new();
        let mut evaluated = 0;

        for id in 0..self.len() {
            evaluated += 1;
            if let Some(metadata) = self.metadata.get(id) {
                if evaluate(filter, &metadata).unwrap_or(false) {
                    passing_ids.push(id);
                }
            }
        }

        let passed = passing_ids.len();
        let selectivity = if evaluated > 0 {
            (passed as f32) / (evaluated as f32)
        } else {
            0.0
        };

        if passing_ids.is_empty() {
            return Ok(FilteredSearchResult {
                results: vec![],
                complete: true,
                observed_selectivity: 0.0,
                strategy_used: FilterStrategy::PreFilter,
                vectors_evaluated: evaluated,
            });
        }

        // Search on filtered subset
        let results = self.search_subset(query, k, &passing_ids)?;

        Ok(FilteredSearchResult {
            results,
            complete: results.len() >= k,
            observed_selectivity: selectivity,
            strategy_used: FilterStrategy::PreFilter,
            vectors_evaluated: evaluated,
        })
    }

    /// Post-filter strategy: search then filter candidates.
    fn search_postfilter(
        &self,
        query: &[f32],
        k: usize,
        filter: &FilterExpr,
        oversample: f32,
    ) -> Result<FilteredSearchResult, FilterError> {
        // Calculate oversampled ef
        let ef_effective = ((k as f32) * oversample).ceil() as usize;
        let ef_effective = ef_effective.min(EF_CAP).max(k);

        // Run HNSW search with oversampled ef
        let candidates = self.search_internal(query, ef_effective)?;

        // Filter candidates
        let mut results = Vec::with_capacity(k);
        let mut passed = 0;
        let evaluated = candidates.len();

        for candidate in candidates {
            if results.len() >= k {
                break;
            }
            if let Some(metadata) = self.metadata.get(candidate.id) {
                if evaluate(filter, &metadata).unwrap_or(false) {
                    results.push(candidate);
                    passed += 1;
                }
            }
        }

        let selectivity = if evaluated > 0 {
            (passed as f32) / (evaluated as f32)
        } else {
            0.0
        };

        Ok(FilteredSearchResult {
            results,
            complete: results.len() >= k,
            observed_selectivity: selectivity,
            strategy_used: FilterStrategy::PostFilter { oversample },
            vectors_evaluated: evaluated,
        })
    }

    /// Hybrid strategy: estimate selectivity, adapt oversample.
    fn search_hybrid(
        &self,
        query: &[f32],
        k: usize,
        filter: &FilterExpr,
        oversample_min: f32,
        oversample_max: f32,
    ) -> Result<FilteredSearchResult, FilterError> {
        // Estimate selectivity
        let estimate = estimate_selectivity(filter, &self.metadata, self.len());

        // Calculate adaptive oversample
        let oversample = calculate_oversample(estimate.selectivity)
            .max(oversample_min)
            .min(oversample_max);

        // Use post-filter with calculated oversample
        let mut result = self.search_postfilter(query, k, filter, oversample)?;
        result.strategy_used = FilterStrategy::Hybrid { oversample_min, oversample_max };
        Ok(result)
    }
}
```

### Acceptance Criteria
- [ ] `search_filtered()` handles all 4 strategies
- [ ] Auto strategy estimates and selects correctly
- [ ] Pre-filter scans all vectors
- [ ] Post-filter respects EF_CAP
- [ ] Hybrid adapts oversample based on selectivity
- [ ] Returns `FilteredSearchResult` with diagnostics
- [ ] `cargo test hnsw::search_filtered` passes

### Test Cases
```rust
#[test]
fn test_search_filtered_no_filter() {
    let index = create_test_index(1000);
    let query = random_vector(384);
    let result = index.search_filtered(&query, 10, None, FilterStrategy::Auto).unwrap();
    assert_eq!(result.results.len(), 10);
    assert!(result.complete);
    assert_eq!(result.observed_selectivity, 1.0);
}

#[test]
fn test_search_filtered_prefilter() {
    let index = create_test_index_with_categories(1000);
    let filter = parse("category = \"gpu\"").unwrap();
    let query = random_vector(384);
    let result = index.search_filtered(&query, 10, Some(&filter), FilterStrategy::PreFilter).unwrap();
    // All results should have category = "gpu"
    for r in &result.results {
        let meta = index.get_metadata(r.id).unwrap();
        assert_eq!(meta.get("category").unwrap().as_str(), "gpu");
    }
}

#[test]
fn test_search_filtered_postfilter() {
    let index = create_test_index_with_categories(1000);
    let filter = parse("category = \"gpu\"").unwrap();
    let query = random_vector(384);
    let result = index.search_filtered(
        &query, 10, Some(&filter),
        FilterStrategy::PostFilter { oversample: 5.0 }
    ).unwrap();
    assert!(result.vectors_evaluated >= 50); // 10 * 5
}

#[test]
fn test_search_filtered_auto_selects_appropriately() {
    let index = create_test_index_with_high_selectivity(1000); // 90% pass
    let filter = parse("active = true").unwrap();
    let query = random_vector(384);
    let result = index.search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto).unwrap();
    // Should have selected pre-filter for high selectivity
    assert_eq!(result.strategy_used, FilterStrategy::PreFilter);
}
```

---

## Task W23.3.4: Edge Case Handlers

### Description
Implement handlers for filter edge cases: tautology, contradiction, empty index, k > matches.

### Hours: 3h

### Specification

**File:** `src/filter/strategy.rs` (continued)

```rust
impl HnswIndex {
    /// Handle filter edge cases before executing search.
    ///
    /// Returns Some(result) if edge case handled, None to proceed normally.
    fn handle_filter_edge_cases(
        &self,
        filter: &FilterExpr,
        k: usize,
    ) -> Result<Option<FilteredSearchResult>, FilterError> {
        // Empty index
        if self.len() == 0 {
            return Ok(Some(FilteredSearchResult {
                results: vec![],
                complete: true,
                observed_selectivity: 0.0,
                strategy_used: FilterStrategy::Auto,
                vectors_evaluated: 0,
            }));
        }

        // Check for tautology (always true)
        if is_tautology(filter) {
            return Ok(None); // Proceed without filter
        }

        // Check for contradiction (always false)
        if is_contradiction(filter) {
            return Ok(Some(FilteredSearchResult {
                results: vec![],
                complete: true,
                observed_selectivity: 0.0,
                strategy_used: FilterStrategy::Auto,
                vectors_evaluated: 0,
            }));
        }

        Ok(None) // No edge case, proceed normally
    }
}

/// Detect tautological filters (always true).
///
/// # Examples
/// - `a OR NOT a`
/// - `x >= 0 OR x < 0` (if x is numeric)
/// - TRUE literal
pub fn is_tautology(filter: &FilterExpr) -> bool {
    match filter {
        FilterExpr::LiteralBool(true) => true,
        FilterExpr::Or(left, right) => {
            // Check for a OR NOT a pattern
            if are_complementary(left, right) {
                return true;
            }
            // Recurse
            is_tautology(left) || is_tautology(right)
        }
        FilterExpr::And(left, right) => {
            // Both sides must be tautologies
            is_tautology(left) && is_tautology(right)
        }
        _ => false,
    }
}

/// Detect contradictory filters (always false).
///
/// # Examples
/// - `a AND NOT a`
/// - `x > 5 AND x < 3`
/// - FALSE literal
pub fn is_contradiction(filter: &FilterExpr) -> bool {
    match filter {
        FilterExpr::LiteralBool(false) => true,
        FilterExpr::And(left, right) => {
            // Check for a AND NOT a pattern
            if are_complementary(left, right) {
                return true;
            }
            // Check for impossible ranges
            if is_impossible_range(left, right) {
                return true;
            }
            // Recurse
            is_contradiction(left) || is_contradiction(right)
        }
        FilterExpr::Or(left, right) => {
            // Both sides must be contradictions
            is_contradiction(left) && is_contradiction(right)
        }
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
/// # Examples
/// - `x > 10 AND x < 5` is impossible
/// - `x >= 10 AND x <= 5` is impossible (unless integer 5..10)
fn is_impossible_range(a: &FilterExpr, b: &FilterExpr) -> bool {
    match (a, b) {
        (
            FilterExpr::Gt(field1, FilterExpr::LiteralInt(v1)) |
            FilterExpr::Ge(field1, FilterExpr::LiteralInt(v1)),
            FilterExpr::Lt(field2, FilterExpr::LiteralInt(v2)) |
            FilterExpr::Le(field2, FilterExpr::LiteralInt(v2)),
        ) if field1 == field2 => {
            v1 >= v2 // e.g., x > 10 AND x < 10
        }
        (
            FilterExpr::Gt(field1, FilterExpr::LiteralFloat(v1)) |
            FilterExpr::Ge(field1, FilterExpr::LiteralFloat(v1)),
            FilterExpr::Lt(field2, FilterExpr::LiteralFloat(v2)) |
            FilterExpr::Le(field2, FilterExpr::LiteralFloat(v2)),
        ) if field1 == field2 => {
            v1 >= v2
        }
        _ => false,
    }
}
```

### Acceptance Criteria
- [ ] Empty index returns empty result
- [ ] Tautology detection works for common patterns
- [ ] Contradiction detection works for common patterns
- [ ] Impossible range detection works
- [ ] Edge cases short-circuit without full evaluation
- [ ] `cargo test filter::strategy::edge_cases` passes

### Test Cases
```rust
#[test]
fn test_is_tautology_true_literal() {
    let filter = FilterExpr::LiteralBool(true);
    assert!(is_tautology(&filter));
}

#[test]
fn test_is_tautology_a_or_not_a() {
    let a = parse("x = 5").unwrap();
    let not_a = FilterExpr::Not(Box::new(a.clone()));
    let or = FilterExpr::Or(Box::new(a), Box::new(not_a));
    assert!(is_tautology(&or));
}

#[test]
fn test_is_contradiction_false_literal() {
    let filter = FilterExpr::LiteralBool(false);
    assert!(is_contradiction(&filter));
}

#[test]
fn test_is_contradiction_a_and_not_a() {
    let a = parse("x = 5").unwrap();
    let not_a = FilterExpr::Not(Box::new(a.clone()));
    let and = FilterExpr::And(Box::new(a), Box::new(not_a));
    assert!(is_contradiction(&and));
}

#[test]
fn test_is_contradiction_impossible_range() {
    let gt10 = parse("x > 10").unwrap();
    let lt5 = parse("x < 5").unwrap();
    let and = FilterExpr::And(Box::new(gt10), Box::new(lt5));
    assert!(is_contradiction(&and));
}

#[test]
fn test_edge_case_empty_index() {
    let index = HnswIndex::new(384);
    let filter = parse("x = 1").unwrap();
    let query = random_vector(384);
    let result = index.search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto).unwrap();
    assert!(result.results.is_empty());
    assert!(result.complete);
}
```

---

## Integration Test Suite

After completing all Day 3 tasks:

```rust
// tests/filter_strategy_integration_tests.rs

#[test]
fn test_full_strategy_pipeline() {
    let index = create_test_index_with_metadata(10_000);
    let query = random_vector(384);

    // Test each strategy
    for strategy in [
        FilterStrategy::Auto,
        FilterStrategy::PreFilter,
        FilterStrategy::PostFilter { oversample: 3.0 },
        FilterStrategy::HYBRID_DEFAULT,
    ] {
        let filter = parse("category = \"electronics\" AND price < 500").unwrap();
        let result = index.search_filtered(&query, 10, Some(&filter), strategy);
        assert!(result.is_ok());

        let result = result.unwrap();
        // Verify all results match filter
        for r in &result.results {
            let meta = index.get_metadata(r.id).unwrap();
            assert_eq!(meta.get("category").unwrap().as_str(), "electronics");
            assert!(meta.get("price").unwrap().as_f64() < 500.0);
        }
    }
}

#[test]
fn test_selectivity_affects_strategy() {
    let mut index = HnswIndex::new(384);

    // Add vectors: 90% active, 10% inactive
    for i in 0..1000 {
        let vector = random_vector(384);
        let metadata = if i < 900 {
            hashmap! { "active" => MetadataValue::Bool(true) }
        } else {
            hashmap! { "active" => MetadataValue::Bool(false) }
        };
        index.add_with_metadata(vector, metadata).unwrap();
    }

    let query = random_vector(384);

    // High selectivity filter (90% pass) should use PreFilter
    let high_sel = parse("active = true").unwrap();
    let result = index.search_filtered(&query, 10, Some(&high_sel), FilterStrategy::Auto).unwrap();
    assert_eq!(result.strategy_used, FilterStrategy::PreFilter);

    // Low selectivity filter (10% pass) should use PostFilter or Hybrid
    let low_sel = parse("active = false").unwrap();
    let result = index.search_filtered(&query, 10, Some(&low_sel), FilterStrategy::Auto).unwrap();
    assert!(matches!(
        result.strategy_used,
        FilterStrategy::PostFilter { .. } | FilterStrategy::Hybrid { .. }
    ));
}
```

---

## Deliverables Checklist

| Artifact | Path | Status |
|:---------|:-----|:-------|
| FilterStrategy enum | `src/filter/strategy.rs` | [ ] |
| Selectivity estimation | `src/filter/strategy.rs` | [ ] |
| search_filtered() API | `src/hnsw/mod.rs` | [ ] |
| Edge case handlers | `src/filter/strategy.rs` | [ ] |
| Unit tests | Inline + `tests/` | [ ] |

---

## End of Day 3 Gate

**Pass Criteria:**
- [ ] All 4 tasks complete
- [ ] `cargo test filter::strategy` passes
- [ ] `cargo test hnsw::search_filtered` passes
- [ ] Integration tests pass
- [ ] No clippy warnings

**Handoff:**
```
[RUST_ENGINEER]: Day 3 Complete

Artifacts generated:
- src/filter/strategy.rs (FilterStrategy, selectivity estimation)
- src/hnsw/mod.rs (search_filtered API)
- tests/filter_strategy_tests.rs

Status: READY_FOR_DAY_4

Next: W23.4.1 (WASM Bindings - parse_filter_js)
```

---

**Day 3 Total: 12 hours | 4 tasks | 408 strategy tests**

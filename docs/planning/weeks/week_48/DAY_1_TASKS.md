# Week 48 — Day 1 Tasks (Monday, Apr 14)

**Date:** 2026-04-14
**Focus:** MetadataBoost Implementation + Unit Tests
**Agents:** RUST_ENGINEER, TEST_ENGINEER
**Status:** PENDING

---

## Day Objective

Implement the MetadataBoost struct and search_boosted() method with full test coverage. By end of day, 11 named unit tests pass and WASM builds.

**Success Criteria:**
- `src/filter/boost.rs` exists with MetadataBoost struct (< 200 lines)
- `search_boosted()` method added to FilteredSearcher
- 11 named unit tests pass with `cargo test test_boost`
- `cargo test --lib` passes (1027+ existing + 11 new)
- `cargo clippy -- -D warnings` clean
- `cargo check --target wasm32-unknown-unknown` succeeds

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `src/filter/filtered_search.rs` — FilteredSearcher pattern, search_filtered() API
- [ ] `src/filter/evaluator.rs` — evaluate() function for metadata matching
- [ ] `src/metadata/types.rs` — MetadataValue enum (5 variants)
- [ ] `src/filter/mod.rs` — current public re-exports
- [ ] `src/hybrid/fusion.rs` — score combination patterns (linear_fusion)
- [ ] `docs/planning/weeks/week_48/WEEKLY_TASK_PLAN.md` — W48 plan for full context

---

## Tasks

### W48.1a: Create `src/filter/boost.rs` — MetadataBoost Struct (3h) — RUST_ENGINEER

**Dependency:** None

**Design:**
```rust
/// Metadata boost configuration for entity-enhanced search.
///
/// A boost increases (or decreases, if weight is negative) the relevance
/// of a search result when a metadata field matches a specified value.
///
/// # Boosting Formula (MULTIPLICATIVE — Scale-Independent)
///
/// `final_distance = raw_distance * (1.0 - boost_factor)`
/// `boost_factor = sum(boost_contributions).clamp(-1.0, 0.99)`
/// `boost_contribution = weight * match(metadata[field], value)`
///
/// Where match() returns 1.0 if the field value matches the boost value,
/// 0.0 otherwise. For StringArray fields, match() uses `contains(value)`.
/// Type mismatches return 0.0 (no error, no panic).
///
/// This is scale-independent: a weight of 0.3 reduces distance by 30%
/// regardless of whether L2 distances are 0.001 or 50000.
#[derive(Clone, Debug)]
pub struct MetadataBoost {
    /// Metadata field name to check (e.g., "entity_type")
    pub field: String,
    /// Value to match against (e.g., MetadataValue::String("ORG"))
    pub value: MetadataValue,
    /// Weight to apply when matched. Positive = boost (reduce distance),
    /// negative = penalty (increase distance), zero = neutral.
    /// Must be finite (NaN/Inf rejected at construction).
    pub weight: f32,
}

/// Error type for MetadataBoost construction.
#[derive(Clone, Debug)]
pub enum BoostError {
    /// Weight is NaN or Infinity. All public entry points validate finiteness.
    NonFiniteWeight(f32),
}
```

**Methods to implement:**
1. `pub fn new(field: impl Into<String>, value: MetadataValue, weight: f32) -> Result<Self, BoostError>`
   - Rejects NaN/Inf weights: `if !weight.is_finite() { return Err(BoostError::NonFiniteWeight(weight)); }`
   - Per CLAUDE.md Section 3.1: "All public entry points validate finiteness"
2. `pub fn compute_boost(&self, metadata: &HashMap<String, MetadataValue>) -> f32`
   - Returns `self.weight` if metadata field matches value:
     - Exact equality: `metadata.get(&self.field) == Some(&self.value)`
     - **StringArray contains:** if metadata field is `StringArray(arr)` and boost value is `String(s)`, returns `self.weight` if `arr.contains(&s)`
   - Returns `0.0` if field missing, value mismatch, or type mismatch
   - NaN weight: return 0.0 (defense-in-depth, though constructor already rejects)
3. `pub fn compute_total_boost(boosts: &[MetadataBoost], metadata: &HashMap<String, MetadataValue>) -> f32`
   - Sum of all individual boost contributions

**Size guard:** MAX 200 lines including tests section marker. If approaching 200, stop and simplify.

**Commands:**
```bash
# Create the file
# (manual — write boost.rs)

# Verify compilation
cargo build
cargo clippy -- -D warnings
```

**Acceptance:**
- [ ] `MetadataBoost` struct with 3 fields (field, value, weight)
- [ ] `BoostError` enum with `NonFiniteWeight` variant
- [ ] `MetadataBoost::new()` returns `Result<Self, BoostError>`, rejects NaN/Inf weights
- [ ] `compute_boost()` returns weight on match, 0.0 on mismatch
- [ ] `compute_boost()` supports StringArray contains matching (StringArray field + String value)
- [ ] `compute_total_boost()` sums contributions
- [ ] NaN weight guard in `compute_boost()` (defense-in-depth, returns 0.0)
- [ ] `cargo build` succeeds
- [ ] File < 200 lines (excluding tests)

---

### W48.1b: Add `search_boosted()` to FilteredSearcher (3h) — RUST_ENGINEER

**Dependency:** W48.1a complete

**Implementation strategy:**
1. Add `use crate::filter::boost::MetadataBoost;` to `filtered_search.rs`
2. Add method to `impl<'idx, 'sto, 'meta, M: MetadataStore> FilteredSearcher`:

```rust
pub fn search_boosted(
    &mut self,
    query: &[f32],
    k: usize,
    boosts: &[MetadataBoost],
    filter: Option<&FilterExpr>,
    strategy: FilterStrategy,
) -> Result<FilteredSearchResult, FilteredSearchError> {
    // Empty boosts = delegate to search_filtered
    if boosts.is_empty() {
        return self.search_filtered(query, k, filter, strategy);
    }

    // Oversample: fetch more candidates for reranking
    // Formula: (k * 3).max(50).min(500)
    // - k*3: enough candidates for meaningful reranking
    // - .max(50): minimum pool size for small k
    // - .min(500): cap to avoid scanning too many candidates
    let oversample_k = (k * 3).max(50).min(500);

    // Get candidates via search_filtered
    let mut result = self.search_filtered(query, oversample_k, filter, strategy)?;

    // Rerank: MULTIPLICATIVE boosting (scale-independent)
    // final_distance = raw_distance * (1.0 - boost_factor)
    // boost_factor = sum(boosts).clamp(-1.0, 0.99)
    //
    // This is scale-independent: weight 0.3 with one match reduces
    // distance by 30%, regardless of whether L2 distances are 0.001
    // or 50000.
    for search_result in &mut result.results {
        let idx = (search_result.vector_id.0 as usize).saturating_sub(1);
        if let Some(meta) = self.metadata.get_metadata(idx) {
            let boost_factor = MetadataBoost::compute_total_boost(boosts, meta)
                .clamp(-1.0, 0.99);
            search_result.distance *= 1.0 - boost_factor;
        }
    }

    // Re-sort by distance (ascending = best first)
    result.results.sort_by(|a, b|
        a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Greater)
    );

    // Truncate to k
    result.results.truncate(k);

    Ok(result)
}
```

**Critical design note — MULTIPLICATIVE BOOSTING (Scale-Independent):**
EdgeVec uses DISTANCE (lower = better). The boost formula is:
```
final_distance = raw_distance * (1.0 - boost_factor)
boost_factor = sum(weight_i * match_i).clamp(-1.0, 0.99)
```
- A positive weight (e.g., 0.3) with a match reduces distance by 30% (makes result "closer")
- A negative weight (e.g., -0.2) with a match increases distance by 20% (makes result "farther")
- The `.clamp(-1.0, 0.99)` prevents distance from going negative or being zeroed out
- This works regardless of L2 scale because it is a percentage reduction, not an absolute subtraction

**Commands:**
```bash
cargo build
cargo clippy -- -D warnings
cargo check --target wasm32-unknown-unknown
```

**Acceptance:**
- [ ] `search_boosted()` method added to FilteredSearcher
- [ ] Empty boosts delegates to search_filtered (no reranking overhead)
- [ ] Oversample formula: `(k * 3).max(50).min(500)` — consistent with weekly plan
- [ ] MULTIPLICATIVE boosting: `final_distance = raw_distance * (1.0 - boost_factor)` where `boost_factor = sum(boosts).clamp(-1.0, 0.99)` — scale-independent
- [ ] Results re-sorted after boosting
- [ ] Results truncated to k
- [ ] `cargo build` succeeds

---

### W48.1c: Write 11 MetadataBoost Unit Tests (2h) — TEST_ENGINEER

**Dependency:** W48.1a, W48.1b complete

**Test list (all 11 required):**

1. `test_boost_single_field_match` — One boost, field matches, distance reduced by weight percentage
2. `test_boost_multiple_fields` — Two boosts on different fields, both apply (multiplicative)
3. `test_boost_no_match_no_effect` — Boost on field that doesn't match, distance unchanged
4. `test_boost_all_match_reranks` — All vectors match boost, relative order preserved
5. `test_boost_weight_zero_neutral` — weight=0.0 has no effect on ranking
6. `test_boost_negative_weight_penalty` — Negative weight increases distance (penalizes)
7. `test_boost_combined_with_filter` — Boost + FilterExpr together work correctly
8. `test_boost_large_weight_dominates` — Very large weight (0.99) nearly zeroes distance
9. `test_boost_type_mismatch_ignored` — Integer boost on String field returns 0.0
10. `test_boost_string_array_contains` — Boost with `String("ORG")` value matches `StringArray(["ORG", "PERSON"])` field via `contains()`. Verifies the demo use case where entity_types is a StringArray.
11. `test_boost_wasm_smoke` — MetadataBoost implements Serialize/Deserialize (for WASM JSON parsing)

**Commands:**
```bash
cargo test test_boost            # Run all boost tests
cargo test --lib                  # Full regression
```

**Acceptance:**
- [ ] All 11 tests listed above pass
- [ ] Tests use `create_test_index()` helper from filtered_search tests or similar
- [ ] Each test has a clear assert on the expected behavior
- [ ] `cargo test test_boost` — 11 passed, 0 failed

---

### W48.1d: Full Regression (0.5h) — TEST_ENGINEER

**Dependency:** W48.1a, W48.1b, W48.1c complete

**Commands:**
```bash
cargo test --lib                                  # 1027+ tests + 11 new
cargo clippy -- -D warnings                       # 0 warnings
cargo check --target wasm32-unknown-unknown        # WASM build
```

**Expected Output:** All green. No regressions.

**Acceptance:**
- [ ] `cargo test --lib` — 1024+ passed (1027 existing + 11 new), 0 failed
- [ ] `cargo clippy -- -D warnings` — 0 warnings
- [ ] `cargo check --target wasm32-unknown-unknown` — success

---

## Day 1 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~8.5h |
| New files | 1 (`src/filter/boost.rs`) |
| New types | 1 (`BoostError` enum) |
| New methods | 3 (new, compute_boost, compute_total_boost) + 1 (search_boosted) |
| New tests | 11 |
| Regressions allowed | 0 |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W48.1a | 3h | | |
| W48.1b | 3h | | |
| W48.1c | 2h | | |
| W48.1d | 0.5h | | |
| **Total** | **8.5h** | | |

---

## Handoff to Day 2

**Codebase state at EOD:**
- `src/filter/boost.rs` exists with MetadataBoost struct
- `src/filter/filtered_search.rs` has search_boosted() method
- 11 boost tests pass
- All existing tests unbroken
- WASM builds

**Day 2 prerequisites satisfied:**
- [ ] MetadataBoost struct exists (needed for WASM export JSON parsing)
- [ ] search_boosted() works (needed for WASM export to call)

**Day 2 focus:** WASM boost export + mid-week hostile review

---

**END OF DAY 1 TASKS**

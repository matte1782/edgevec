# Week 37: Sparse Vectors Phase 1 — Core Types

**Date Range:** 2026-01-12 to 2026-01-18
**Focus:** Implement SparseVector and metrics (Foundation for v0.9.0)
**Hours:** 12h (2h/day)
**Status:** [ ] PROPOSED
**Depends On:** RFC-007 (APPROVED), SPARSE_MODULE_STRUCTURE.md

---

## Context

Week 37 begins Phase 1 of sparse vector implementation as defined in RFC-007. This week focuses on the foundational types and metrics that all subsequent sparse functionality will build upon.

**RFC-007 Phase 1 Goals:**
- Define `SparseVector` with CSR format
- Implement validation (sorted indices, no duplicates, no NaN)
- Implement `sparse_dot_product` and `sparse_cosine`
- Property tests for metric correctness

**Performance Targets (from RFC-007):**
- Dot product (50 nnz): P50 <300ns, P99 <500ns
- Dot product (100 nnz): P50 <600ns, P99 <1μs

---

## Week 37 Tasks Overview

| Day | Task | Hours | Priority |
|:----|:-----|:------|:---------|
| 1 | Create `src/sparse/` module structure | 2h | P0 |
| 2 | Implement `SparseVector` struct | 2h | P0 |
| 3 | Implement sparse metrics | 2h | P0 |
| 4 | Property tests for SparseVector | 2h | P0 |
| 5 | Property tests for metrics | 2h | P0 |
| 6 | Benchmarks + hostile review | 2h | P0 |
| 7 | Buffer | - | - |

**Total:** 12 hours

---

## Day 1: Module Structure (2h)

**Goal:** Create the sparse module skeleton with proper exports

### Tasks

- [ ] **1.1** Create `src/sparse/mod.rs` (30min)
  - Define module structure
  - Add feature flag `sparse` (default enabled)
  - Export placeholder types

- [ ] **1.2** Create `src/sparse/error.rs` (30min)
  - Define `SparseError` enum with variants:
    - `UnsortedIndices`
    - `DuplicateIndex(usize)`
    - `IndexOutOfBounds { index: u32, dim: u32 }`
    - `InvalidValue(usize)` (NaN/Infinity)
    - `EmptyVector`
    - `LengthMismatch { indices: usize, values: usize }`
    - `IdNotFound(u64)`
    - `ZeroNorm`

- [ ] **1.3** Update `src/lib.rs` (30min)
  - Add `pub mod sparse;`
  - Re-export core types: `SparseVector`, `SparseError`
  - Verify module compiles

- [ ] **1.4** Create placeholder files (30min)
  - `src/sparse/vector.rs` — empty struct
  - `src/sparse/metrics.rs` — empty functions
  - `src/sparse/storage.rs` — placeholder for Week 38
  - `src/sparse/search.rs` — placeholder for Week 39

### Acceptance Criteria

- [ ] `cargo check --features sparse` passes
- [ ] `SparseError` has all 8 variants from RFC-007
- [ ] Module structure matches `SPARSE_MODULE_STRUCTURE.md`

---

## Day 2: SparseVector Implementation (2h)

**Goal:** Implement `SparseVector` with full validation

### Tasks

- [ ] **2.1** Define `SparseVector` struct (30min)
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
  pub struct SparseVector {
      indices: Vec<u32>,
      values: Vec<f32>,
      dim: u32,
  }
  ```

- [ ] **2.2** Implement constructors (45min)
  - `new(indices, values, dim)` — validates sorted, no dups, no NaN, nnz >= 1
  - `from_pairs(pairs, dim)` — sorts internally, then validates
  - `singleton(index, value, dim)` — single element

- [ ] **2.3** Implement accessors (15min)
  - `indices(&self) -> &[u32]`
  - `values(&self) -> &[f32]`
  - `dim(&self) -> u32`
  - `nnz(&self) -> usize`

- [ ] **2.4** Implement validation helper (30min)
  ```rust
  fn validate(indices: &[u32], values: &[f32], dim: u32) -> Result<(), SparseError>
  ```
  - Check indices sorted
  - Check no duplicates
  - Check no NaN/Infinity
  - Check nnz >= 1
  - Check all indices < dim

### Acceptance Criteria

- [ ] `SparseVector::new` validates all invariants
- [ ] `SparseVector::from_pairs` auto-sorts
- [ ] Invalid input returns appropriate `SparseError`
- [ ] Unit tests for all validation cases

---

## Day 3: Sparse Metrics (2h)

**Goal:** Implement dot product and cosine similarity

### Tasks

- [ ] **3.1** Implement `sparse_dot_product` (45min)
  ```rust
  /// O(|a| + |b|) merge-intersection algorithm
  pub fn sparse_dot_product(a: &SparseVector, b: &SparseVector) -> f32
  ```

- [ ] **3.2** Implement `sparse_norm` (15min)
  ```rust
  /// L2 norm: sqrt(sum(v_i^2))
  pub fn sparse_norm(v: &SparseVector) -> f32
  ```

- [ ] **3.3** Implement `sparse_cosine` (30min)
  ```rust
  /// Cosine similarity: dot(a,b) / (||a|| * ||b||)
  /// Returns 0.0 for zero vectors
  pub fn sparse_cosine(a: &SparseVector, b: &SparseVector) -> f32
  ```

- [ ] **3.4** Add method wrappers to `SparseVector` (30min)
  - `fn dot(&self, other: &SparseVector) -> f32`
  - `fn cosine(&self, other: &SparseVector) -> f32`
  - `fn norm(&self) -> f32`
  - `fn normalize(&self) -> Result<SparseVector, SparseError>`

### Acceptance Criteria

- [ ] `sparse_dot_product` matches dense computation for same vectors
- [ ] `sparse_cosine` returns value in [-1, 1] range
- [ ] `normalize` returns unit vector (norm = 1.0)
- [ ] Unit tests pass

---

## Day 4: SparseVector Property Tests (2h)

**Goal:** Verify SparseVector invariants with proptest

### Tasks

- [ ] **4.1** Create test file structure (15min)
  - `tests/sparse_vector_test.rs`
  - Add proptest dependency if not present

- [ ] **4.2** Write unit tests (45min)
  - `test_new_valid` — sorted indices, valid values
  - `test_new_unsorted_fails` — error on unsorted
  - `test_new_duplicate_fails` — error on duplicate index
  - `test_new_nan_fails` — error on NaN value
  - `test_new_empty_fails` — error on zero nnz
  - `test_from_pairs_sorts` — auto-sorting
  - `test_singleton` — single element

- [ ] **4.3** Write property tests (1h)
  - Arbitrary `SparseVector` generator
  - Property: indices always sorted after construction
  - Property: nnz matches indices.len()
  - Property: all values are finite
  - Property: from_pairs(to_pairs(v)) == v

### Acceptance Criteria

- [ ] All unit tests pass
- [ ] Property tests run 1000+ cases
- [ ] No test flakiness

---

## Day 5: Metrics Property Tests (2h)

**Goal:** Verify metric correctness with proptest

### Tasks

- [ ] **5.1** Create metrics test file (15min)
  - `tests/sparse_metrics_test.rs`

- [ ] **5.2** Write unit tests (45min)
  - `test_dot_product_zero` — orthogonal vectors
  - `test_dot_product_self` — v · v = ||v||²
  - `test_cosine_self_is_one` — cos(v, v) = 1.0
  - `test_cosine_orthogonal_is_zero`
  - `test_normalize_has_unit_norm`

- [ ] **5.3** Write property tests (1h)
  - Property: `dot(a, b) == dot(b, a)` (commutativity)
  - Property: `dot(a, a) >= 0` (positive semi-definite)
  - Property: `cosine(a, a) == 1.0` for non-zero vectors
  - Property: `-1.0 <= cosine(a, b) <= 1.0`
  - Property: `norm(normalize(v)) ≈ 1.0`

### Acceptance Criteria

- [ ] All property tests pass with 1000+ cases
- [ ] Dot product matches dense computation (cross-check)
- [ ] Cosine similarity always in valid range

---

## Day 6: Benchmarks + Review (2h)

**Goal:** Validate performance targets and get hostile review

### Tasks

- [ ] **6.1** Create benchmark file (45min)
  - `benches/sparse_bench.rs`
  - `bench_dot_50_nnz` — target P99 <500ns
  - `bench_dot_100_nnz` — target P99 <1μs
  - `bench_dot_500_nnz` — measure scaling

- [ ] **6.2** Run benchmarks (30min)
  - `cargo bench --bench sparse_bench`
  - Document results
  - Compare to RFC-007 targets

- [ ] **6.3** Submit for hostile review (45min)
  - `/review src/sparse/`
  - Address any issues
  - Document review outcome

### Acceptance Criteria

- [ ] Dot product (50 nnz) P99 < 500ns
- [ ] Dot product (100 nnz) P99 < 1μs
- [ ] Hostile review APPROVED
- [ ] All tests pass: `cargo test --features sparse`

---

## Day 7: Buffer

Reserved for:
- Addressing hostile review feedback
- Performance optimization if needed
- Spillover from Days 1-6

---

## Dependencies

| Dependency | Status | Impact |
|:-----------|:-------|:-------|
| RFC-007 | APPROVED | Design complete |
| SPARSE_MODULE_STRUCTURE.md | EXISTS | Module layout defined |
| proptest crate | EXISTS | Already in Cargo.toml |
| criterion crate | EXISTS | Already in Cargo.toml |

---

## Success Metrics

| Metric | Target |
|:-------|:-------|
| `SparseVector` implementation | Complete |
| `sparse_dot_product` implementation | Complete |
| `sparse_cosine` implementation | Complete |
| Property tests | 5+ properties, 1000+ cases each |
| Unit tests | 15+ tests |
| Benchmark results | Within RFC-007 targets |
| Hostile review | APPROVED |

---

## Risk Mitigation

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| Performance below target | LOW | Profile early, optimize hot path |
| proptest generator complexity | LOW | Start simple, add constraints |
| Validation edge cases | MEDIUM | Comprehensive unit tests |

---

## Files Created This Week

```
src/sparse/
├── mod.rs           # Module exports
├── error.rs         # SparseError enum
├── vector.rs        # SparseVector struct
└── metrics.rs       # dot_product, cosine, norm

tests/
├── sparse_vector_test.rs
└── sparse_metrics_test.rs

benches/
└── sparse_bench.rs
```

---

## Commit Message Template

```
feat(sparse): implement SparseVector core types (Week 37)

- Add src/sparse/ module with SparseVector, SparseError
- Implement sparse_dot_product, sparse_cosine, sparse_norm
- Property tests verify commutativity, positive semi-definite
- Benchmarks validate <500ns dot product (50 nnz)

RFC-007 Phase 1 complete. Ready for Phase 2 (Storage).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

## Week 37 Exit Criteria

Week 37 is complete when:
- [ ] `SparseVector` fully implemented with validation
- [ ] All 3 metric functions implemented
- [ ] Property tests pass (1000+ cases)
- [ ] Benchmarks meet RFC-007 targets
- [ ] Hostile review APPROVED
- [ ] Week 38 plan created

---

## @jsonMartin Status

**Status:** No Flat Index RFC received as of Week 36 end.

**Action:** Continue with sparse vectors (no external dependency). Will re-check at Week 38 planning.

**Note:** If RFC arrives during Week 37, it can be integrated into Week 39+ planning without blocking sparse work.

---

**Agent:** PLANNER
**Hours:** 12h total
**Priority:** P0 (v0.9.0 core feature)

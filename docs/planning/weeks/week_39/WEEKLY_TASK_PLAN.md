# Week 39: RRF Hybrid Search Implementation

**Date Range:** 2026-01-26 to 2026-02-01
**Focus:** Implement RRF Hybrid Search (Milestone 9.2)
**Hours:** 16h (2-3h/day)
**Status:** [ ] PROPOSED
**Depends On:** Week 38 COMPLETE (SparseStorage, 67 sparse tests passing), RFC-007 Phase 2

---

## Context

Week 39 implements the core hybrid search capability that combines dense (HNSW) and sparse (BM25/TF-IDF) search results. This is the #1 community-requested feature per Lucas (Reddit) asking for BM25/hybrid search.

**RFC-007 Phase 3 Goals:**
- Implement `SparseSearcher` with brute-force search over `SparseStorage`
- Implement RRF (Reciprocal Rank Fusion) algorithm
- Implement Linear combination fusion mode
- Create `hybridSearch()` API that orchestrates dense + sparse
- Add WASM bindings and TypeScript helpers

**From ROADMAP v6.1 (Milestone 9.2):**

| Feature | Hours | Description |
|:--------|:------|:------------|
| RRF fusion algorithm | 4h | Reciprocal Rank Fusion (k=60 default) |
| `hybridSearch()` API | 6h | Combines dense + sparse results |
| Linear combination mode | 4h | alpha-weighted score fusion |
| TypeScript helpers | 2h | Easy integration |

**API Design (from ROADMAP):**
```typescript
// JavaScript API
const hybridResults = await db.hybridSearch({
  dense: { vector: embedding, k: 20 },
  sparse: { vector: bm25Scores, k: 20 },
  fusion: 'rrf',  // or { type: 'linear', alpha: 0.7 }
  k: 10
});
```

**Exit Criteria (from ROADMAP):**
- [ ] RRF recall >0.90 on standard benchmark
- [ ] Linear fusion mode tested
- [ ] Integration tests with real BM25 scores

---

## Week 39 Tasks Overview

| Day | Date | Task | Hours | Priority |
|:----|:-----|:-----|:------|:---------|
| 1 | 2026-01-26 | SparseSearcher struct + brute force search | 3h | P0 |
| 2 | 2026-01-27 | RRF fusion algorithm implementation | 3h | P0 |
| 3 | 2026-01-28 | hybridSearch() API + HnswIndex integration | 3h | P0 |
| 4 | 2026-01-29 | Linear combination fusion mode | 2h | P0 |
| 5 | 2026-01-30 | WASM bindings + TypeScript types | 3h | P0 |
| 6 | 2026-02-01 | Integration tests + Benchmarks + Hostile Review | 2h | P0 |

**Total:** 16 hours

---

## Day 1: SparseSearcher Implementation (3h)

**Date:** 2026-01-26
**Goal:** Create `src/sparse/search.rs` with brute-force sparse search

### Tasks

#### W39.1.1: Create `SparseSearcher` struct (1h)

**Objective:** Define the sparse search engine that queries `SparseStorage`.

**File:** `src/sparse/search.rs`

```rust
/// Sparse vector search engine.
///
/// Performs brute-force search over a `SparseStorage` using sparse dot product.
/// For small collections (<100k vectors), brute force is efficient due to
/// sparse vector locality.
///
/// # Performance
///
/// - Search: O(n * avg_nnz) where n is live vector count
/// - Target: <10ms for 10k vectors with avg 50 nnz
pub struct SparseSearcher<'a> {
    storage: &'a SparseStorage,
}
```

**Acceptance Criteria:**
- [ ] `SparseSearcher` struct defined with storage reference
- [ ] Lifetime annotation for borrowed storage
- [ ] Doc comments with performance characteristics

---

#### W39.1.2: Implement `search()` method (1.5h)

**Objective:** Brute-force top-k search using sparse dot product.

```rust
impl<'a> SparseSearcher<'a> {
    pub fn new(storage: &'a SparseStorage) -> Self;

    /// Search for top-k most similar sparse vectors.
    ///
    /// Uses sparse dot product similarity (higher = more similar).
    ///
    /// # Arguments
    /// * `query` - Sparse query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    /// Vec of (SparseId, score) pairs, sorted by descending score.
    pub fn search(&self, query: &SparseVector, k: usize) -> Vec<(SparseId, f32)>;
}
```

**Acceptance Criteria:**
- [ ] Returns top-k results sorted by descending score
- [ ] Skips deleted vectors
- [ ] Handles k > live_count gracefully
- [ ] O(n * avg_nnz) complexity

---

#### W39.1.3: Unit tests for SparseSearcher (30min)

**Acceptance Criteria:**
- [ ] `test_search_basic` - Basic search works
- [ ] `test_search_empty_storage` - Returns empty vec
- [ ] `test_search_k_larger_than_count` - Returns all available
- [ ] `test_search_skips_deleted` - Deleted vectors excluded
- [ ] `test_search_ordering` - Results sorted by score descending

---

## Day 2: RRF Fusion Algorithm (3h)

**Date:** 2026-01-27
**Goal:** Implement Reciprocal Rank Fusion

### Tasks

#### W39.2.1: Create `src/hybrid/mod.rs` module (30min)

**Objective:** New module for hybrid search functionality.

```rust
// src/hybrid/mod.rs
mod fusion;
mod search;

pub use fusion::{rrf_fusion, linear_fusion, FusionMethod};
pub use search::HybridSearcher;
```

---

#### W39.2.2: Implement `rrf_fusion()` (1.5h)

**Objective:** RRF algorithm implementation.

**File:** `src/hybrid/fusion.rs`

```rust
/// Reciprocal Rank Fusion (RRF) algorithm.
///
/// Combines two ranked lists into a single ranking using the formula:
/// score(d) = sum(1 / (k + rank_i(d))) for each list i
///
/// # Algorithm
///
/// For each document appearing in either list:
/// 1. Look up its rank in the dense results (1-indexed, or infinity if absent)
/// 2. Look up its rank in the sparse results (1-indexed, or infinity if absent)
/// 3. Compute RRF score: 1/(k + rank_dense) + 1/(k + rank_sparse)
///
/// # Arguments
///
/// * `dense_results` - Results from dense search (id, score) ordered by descending score
/// * `sparse_results` - Results from sparse search (id, score) ordered by descending score
/// * `k` - RRF constant (default 60, higher = smoother fusion)
/// * `top_n` - Number of results to return
///
/// # Returns
///
/// Vec of (id, rrf_score) pairs, sorted by descending RRF score.
///
/// # Reference
///
/// Cormack et al. "Reciprocal Rank Fusion outperforms Condorcet and
/// individual Rank Learning Methods" (SIGIR 2009)
pub fn rrf_fusion(
    dense_results: &[(u64, f32)],
    sparse_results: &[(u64, f32)],
    k: u32,
    top_n: usize,
) -> Vec<(u64, f32)>
```

**Acceptance Criteria:**
- [ ] Correct RRF formula: 1/(k + rank)
- [ ] Handles documents in only one list
- [ ] Returns sorted by descending RRF score
- [ ] Default k=60 matches industry standard

---

#### W39.2.3: Unit tests for RRF (1h)

**Acceptance Criteria:**
- [ ] `test_rrf_identical_lists` - Same results yield high scores
- [ ] `test_rrf_disjoint_lists` - No overlap handled
- [ ] `test_rrf_partial_overlap` - Mixed case
- [ ] `test_rrf_k_parameter` - Different k values
- [ ] `test_rrf_empty_lists` - Edge case handling

---

## Day 3: HybridSearch API (3h)

**Date:** 2026-01-28
**Goal:** Create unified hybrid search interface

### Tasks

#### W39.3.1: Create `HybridSearcher` struct (1h)

**File:** `src/hybrid/search.rs`

```rust
/// Hybrid search combining dense and sparse retrieval.
///
/// Orchestrates HNSW graph search and sparse storage search,
/// then fuses results using RRF or linear combination.
pub struct HybridSearcher<'a> {
    graph: &'a HnswGraph,
    dense_storage: &'a VectorStorage,
    sparse_storage: &'a SparseStorage,
}

/// Configuration for hybrid search.
pub struct HybridSearchConfig {
    /// Number of results to retrieve from dense search
    pub dense_k: usize,
    /// Number of results to retrieve from sparse search
    pub sparse_k: usize,
    /// Final number of results to return
    pub final_k: usize,
    /// Fusion method to use
    pub fusion: FusionMethod,
}

/// Fusion method for combining results.
pub enum FusionMethod {
    /// Reciprocal Rank Fusion with k parameter
    Rrf { k: u32 },
    /// Linear combination with alpha weight
    /// final_score = alpha * dense_score + (1 - alpha) * sparse_score
    Linear { alpha: f32 },
}
```

---

#### W39.3.2: Implement `hybrid_search()` method (1.5h)

```rust
impl<'a> HybridSearcher<'a> {
    pub fn new(
        graph: &'a HnswGraph,
        dense_storage: &'a VectorStorage,
        sparse_storage: &'a SparseStorage,
    ) -> Self;

    /// Perform hybrid search combining dense and sparse retrieval.
    ///
    /// # Arguments
    /// * `dense_query` - Dense embedding vector
    /// * `sparse_query` - Sparse keyword vector
    /// * `config` - Search configuration
    ///
    /// # Returns
    /// Vec of (VectorId, combined_score) pairs.
    pub fn search(
        &self,
        dense_query: &[f32],
        sparse_query: &SparseVector,
        config: &HybridSearchConfig,
    ) -> Result<Vec<(VectorId, f32)>, GraphError>;
}
```

**Acceptance Criteria:**
- [ ] Executes dense search via HNSW
- [ ] Executes sparse search via SparseSearcher
- [ ] Fuses results using configured method
- [ ] Returns combined results

---

#### W39.3.3: Integration tests (30min)

**Acceptance Criteria:**
- [ ] `test_hybrid_search_basic` - End-to-end works
- [ ] `test_hybrid_rrf_fusion` - RRF mode works
- [ ] `test_hybrid_linear_fusion` - Linear mode works
- [ ] `test_hybrid_no_sparse_matches` - Graceful handling

---

## Day 4: Linear Combination Fusion (2h)

**Date:** 2026-01-29
**Goal:** Implement alpha-weighted linear fusion

### Tasks

#### W39.4.1: Implement `linear_fusion()` (1h)

**File:** `src/hybrid/fusion.rs`

```rust
/// Linear combination fusion.
///
/// Combines scores using: final = alpha * dense_score + (1 - alpha) * sparse_score
///
/// # Score Normalization
///
/// Before combination, scores are min-max normalized to [0, 1]:
/// normalized = (score - min) / (max - min)
///
/// # Arguments
///
/// * `dense_results` - Results from dense search with similarity scores
/// * `sparse_results` - Results from sparse search with dot product scores
/// * `alpha` - Weight for dense scores (0.0 = sparse only, 1.0 = dense only)
/// * `top_n` - Number of results to return
///
/// # Returns
///
/// Vec of (id, combined_score) pairs, sorted by descending score.
pub fn linear_fusion(
    dense_results: &[(u64, f32)],
    sparse_results: &[(u64, f32)],
    alpha: f32,
    top_n: usize,
) -> Vec<(u64, f32)>
```

**Acceptance Criteria:**
- [ ] Score normalization to [0, 1]
- [ ] Correct alpha weighting
- [ ] Alpha=0.0 equals sparse only
- [ ] Alpha=1.0 equals dense only
- [ ] Handles missing documents in one list

---

#### W39.4.2: Unit tests for linear fusion (1h)

**Acceptance Criteria:**
- [ ] `test_linear_alpha_zero` - Pure sparse
- [ ] `test_linear_alpha_one` - Pure dense
- [ ] `test_linear_alpha_half` - Balanced
- [ ] `test_linear_normalization` - Scores normalized
- [ ] `test_linear_disjoint` - No overlap handled

---

## Day 5: WASM Bindings + TypeScript (3h)

**Date:** 2026-01-30
**Goal:** Expose hybrid search to JavaScript

### Tasks

#### W39.5.1: Add sparse search WASM binding (1h)

**File:** `src/wasm/mod.rs`

```rust
/// Search sparse vectors by query.
///
/// # Arguments
/// * `indices` - Uint32Array of sparse indices
/// * `values` - Float32Array of sparse values
/// * `dim` - Dimension of the sparse space
/// * `k` - Number of results
///
/// # Returns
/// JSON string: [{ "id": number, "score": number }, ...]
#[wasm_bindgen]
pub fn search_sparse(
    &self,
    indices: Uint32Array,
    values: Float32Array,
    dim: u32,
    k: usize,
) -> Result<String, JsValue>;
```

---

#### W39.5.2: Add hybrid search WASM binding (1.5h)

```rust
/// Perform hybrid search combining dense and sparse.
///
/// # Arguments
/// * `dense_query` - Float32Array dense embedding
/// * `sparse_indices` - Uint32Array sparse indices
/// * `sparse_values` - Float32Array sparse values
/// * `sparse_dim` - Dimension of sparse space
/// * `options_json` - JSON config string
///
/// # Options JSON Schema
/// ```json
/// {
///   "dense_k": 20,
///   "sparse_k": 20,
///   "k": 10,
///   "fusion": "rrf" | { "type": "linear", "alpha": 0.7 }
/// }
/// ```
#[wasm_bindgen]
pub fn hybrid_search(
    &mut self,
    dense_query: Float32Array,
    sparse_indices: Uint32Array,
    sparse_values: Float32Array,
    sparse_dim: u32,
    options_json: &str,
) -> Result<String, JsValue>;
```

---

#### W39.5.3: TypeScript type definitions (30min)

**File:** `pkg/edgevec.d.ts` additions

```typescript
export interface SparseVector {
  indices: Uint32Array;
  values: Float32Array;
  dim: number;
}

export interface HybridSearchOptions {
  dense_k?: number;
  sparse_k?: number;
  k: number;
  fusion: 'rrf' | { type: 'linear'; alpha: number };
}

export interface HybridSearchResult {
  id: number;
  score: number;
  dense_rank?: number;
  sparse_rank?: number;
}
```

---

## Day 6: Integration Tests + Benchmarks + Review (2h)

**Date:** 2026-02-01
**Goal:** Validate hybrid search quality and performance

### Tasks

#### W39.6.1: Create benchmark suite (45min)

**File:** `benches/hybrid_bench.rs`

**Benchmarks:**
- `bench_sparse_search_10k` - Brute force search over 10k sparse vectors
- `bench_rrf_fusion_1k` - RRF on 1k result lists
- `bench_hybrid_search_10k` - Full hybrid pipeline

**Performance Targets:**
- Sparse search (10k, k=100): <10ms
- RRF fusion (1k results): <1ms
- Hybrid search (10k): <20ms

---

#### W39.6.2: Recall benchmark (30min)

**Objective:** Validate RRF recall > 0.90 on synthetic data.

```rust
#[test]
fn test_rrf_recall_benchmark() {
    // Create synthetic ground truth
    // Generate dense + sparse queries
    // Measure recall@10 for hybrid vs ground truth
    // Assert recall > 0.90
}
```

---

#### W39.6.3: Submit for hostile review (45min)

**Review Command:**
```
/review src/sparse/search.rs src/hybrid/mod.rs src/wasm/mod.rs
```

**Review Checklist:**
- [ ] `SparseSearcher` correctly implements brute force
- [ ] RRF formula matches paper
- [ ] Linear fusion normalizes scores correctly
- [ ] WASM bindings validate inputs
- [ ] Error handling comprehensive
- [ ] All tests pass
- [ ] Benchmarks meet targets
- [ ] No clippy warnings

---

## Week 39 Deliverables

### Files Created

```
src/sparse/
└── search.rs           # SparseSearcher implementation

src/hybrid/
├── mod.rs              # Module exports
├── fusion.rs           # RRF + Linear fusion algorithms
└── search.rs           # HybridSearcher implementation

benches/
└── hybrid_bench.rs     # Hybrid search benchmarks

tests/
├── sparse_search_test.rs    # Sparse search unit tests
└── hybrid_search_test.rs    # Hybrid search integration tests
```

### Files Modified

```
src/lib.rs              # Add hybrid module export
src/sparse/mod.rs       # Uncomment search module
src/wasm/mod.rs         # Add hybrid_search(), search_sparse()
pkg/edgevec.d.ts        # TypeScript types
```

---

## Week 39 Risk Register

| ID | Risk | Likelihood | Impact | Mitigation |
|:---|:-----|:-----------|:-------|:-----------|
| R39.1 | Sparse search too slow at scale | MEDIUM | HIGH | Consider inverted index in v0.10.0 |
| R39.2 | RRF recall below target | LOW | MEDIUM | Tune k parameter, add calibration |
| R39.3 | Score normalization issues | MEDIUM | MEDIUM | Test with diverse score ranges |
| R39.4 | WASM binding complexity | LOW | LOW | Follow existing patterns |
| R39.5 | TypeScript type conflicts | LOW | LOW | Careful export naming |

---

## Week 39 Exit Criteria

Week 39 is complete when:
- [ ] `SparseSearcher` fully implemented with brute-force search
- [ ] RRF fusion algorithm matches paper formula
- [ ] Linear combination fusion with score normalization
- [ ] `HybridSearcher` orchestrates both search types
- [ ] WASM bindings for `search_sparse()` and `hybrid_search()`
- [ ] TypeScript types for all new APIs
- [ ] RRF recall > 0.90 on benchmark
- [ ] All unit tests pass
- [ ] Benchmarks meet performance targets
- [ ] HOSTILE_REVIEWER APPROVED
- [ ] Week 40 plan created

---

## Week 39 Handoff

### To Week 40

**Completed:**
- `SparseSearcher` with brute-force search
- RRF and Linear fusion algorithms
- `HybridSearcher` combining dense + sparse
- WASM + TypeScript integration

**Ready for Week 40:**
- Flat Index implementation (if @jsonMartin RFC received)
- OR: Advanced sparse optimizations (inverted index)

**Dependencies for Week 40:**
- `HybridSearcher` for Flat Index integration (optional)
- Benchmark baselines for comparison

---

## Commit Message Template

```
feat(hybrid): implement RRF hybrid search (Week 39)

- Add SparseSearcher with brute-force top-k search
- Implement RRF fusion algorithm (k=60 default)
- Implement linear combination fusion with alpha
- Add HybridSearcher combining dense + sparse
- WASM bindings: search_sparse(), hybrid_search()
- TypeScript types for hybrid search API

RFC-007 Phase 3 complete. RRF recall > 0.90 validated.

Closes #[issue] Hybrid Search Request

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

**Agent:** PLANNER
**Hours:** 16h total
**Priority:** P0 (v0.9.0 core feature)
**Status:** [PROPOSED]

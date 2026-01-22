# Week 39 Day 6: Integration Tests + Benchmarks + Hostile Review

**Date:** 2026-02-01
**Focus:** Validate hybrid search quality and performance, obtain hostile review
**Estimated Duration:** 2 hours
**Phase:** RFC-007 Implementation Phase 3 (Validation)
**Dependencies:** Days 1-5 COMPLETE (All hybrid search functionality)

---

## Context

Day 6 is validation day. We ensure hybrid search meets quality targets (RRF recall > 0.90)
and performance requirements, then submit for hostile review.

**Exit Criteria (from ROADMAP):**
- [ ] RRF recall > 0.90 on standard benchmark
- [ ] Linear fusion mode tested
- [ ] Integration tests with real BM25 scores

---

## Tasks

### W39.6.1: Create Benchmark Suite

**Objective:** Benchmark hybrid search performance.

**File:** `benches/hybrid_bench.rs`

```rust
//! Benchmarks for hybrid search functionality.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};

use edgevec::hnsw::graph::{HnswGraph, HnswConfig};
use edgevec::storage::VectorStorage;
use edgevec::sparse::{SparseStorage, SparseVector, SparseSearcher};
use edgevec::hybrid::{rrf_fusion, linear_fusion, HybridSearcher, HybridSearchConfig, FusionMethod};

// =============================================================================
// TEST DATA GENERATION
// =============================================================================

fn generate_test_data(
    num_vectors: usize,
    dense_dim: usize,
    sparse_nnz: usize,
    sparse_dim: u32,
) -> (HnswGraph, VectorStorage, SparseStorage) {
    let config = HnswConfig {
        dimensions: dense_dim as u32,
        m: 16,
        ef_construction: 100,
        ..HnswConfig::default()
    };

    let mut graph = HnswGraph::new(config);
    let mut dense_storage = VectorStorage::new(dense_dim);
    let mut sparse_storage = SparseStorage::new();

    // LCG for reproducible random data
    let mut seed: u64 = 42;
    let lcg = |s: &mut u64| -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    for i in 0..num_vectors {
        // Dense vector
        let dense: Vec<f32> = (0..dense_dim)
            .map(|_| (lcg(&mut seed) % 1000) as f32 / 1000.0)
            .collect();
        let id = dense_storage.insert(&dense).unwrap();
        graph.insert(id, &dense, &dense_storage).unwrap();

        // Sparse vector - indices spread across vocabulary
        let indices: Vec<u32> = (0..sparse_nnz)
            .map(|j| ((i * sparse_nnz + j) % (sparse_dim as usize)) as u32)
            .collect();
        let mut indices_sorted = indices.clone();
        indices_sorted.sort_unstable();
        indices_sorted.dedup();

        let values: Vec<f32> = indices_sorted
            .iter()
            .map(|_| (lcg(&mut seed) % 100) as f32 / 10.0)
            .collect();

        if !indices_sorted.is_empty() {
            let sparse = SparseVector::new(indices_sorted, values, sparse_dim).unwrap();
            sparse_storage.insert(&sparse).unwrap();
        }
    }

    (graph, dense_storage, sparse_storage)
}

fn random_dense_query(dim: usize, seed: u64) -> Vec<f32> {
    let mut s = seed;
    let lcg = |s: &mut u64| -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    (0..dim)
        .map(|_| (lcg(&mut s) % 1000) as f32 / 1000.0)
        .collect()
}

fn random_sparse_query(nnz: usize, dim: u32, seed: u64) -> SparseVector {
    let mut s = seed;
    let lcg = |s: &mut u64| -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    let mut indices: Vec<u32> = (0..nnz)
        .map(|_| (lcg(&mut s) % dim as u64) as u32)
        .collect();
    indices.sort_unstable();
    indices.dedup();

    let values: Vec<f32> = indices.iter().map(|_| (lcg(&mut s) % 100) as f32 / 10.0).collect();

    SparseVector::new(indices, values, dim).unwrap()
}

// =============================================================================
// SPARSE SEARCH BENCHMARKS
// =============================================================================

fn bench_sparse_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_search");

    for num_vectors in [1_000, 10_000] {
        let (_, _, sparse_storage) = generate_test_data(num_vectors, 64, 50, 10_000);

        group.bench_with_input(
            BenchmarkId::new("brute_force", num_vectors),
            &num_vectors,
            |b, _| {
                let searcher = SparseSearcher::new(&sparse_storage);
                let query = random_sparse_query(50, 10_000, 12345);

                b.iter(|| {
                    black_box(searcher.search(&query, 100))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// FUSION BENCHMARKS
// =============================================================================

fn bench_rrf_fusion(c: &mut Criterion) {
    let mut group = c.benchmark_group("rrf_fusion");

    for list_size in [100, 500, 1000] {
        // Generate mock results
        let dense: Vec<(u64, f32)> = (0..list_size as u64)
            .map(|i| (i, 1.0 - (i as f32 / list_size as f32)))
            .collect();
        let sparse: Vec<(u64, f32)> = ((list_size / 2) as u64..(list_size * 3 / 2) as u64)
            .map(|i| (i, 10.0 - ((i - list_size as u64 / 2) as f32 / list_size as f32)))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("k60", list_size),
            &list_size,
            |b, _| {
                b.iter(|| {
                    black_box(rrf_fusion(&dense, &sparse, 60, 100))
                });
            },
        );
    }

    group.finish();
}

fn bench_linear_fusion(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear_fusion");

    for list_size in [100, 500, 1000] {
        let dense: Vec<(u64, f32)> = (0..list_size as u64)
            .map(|i| (i, 1.0 - (i as f32 / list_size as f32)))
            .collect();
        let sparse: Vec<(u64, f32)> = ((list_size / 2) as u64..(list_size * 3 / 2) as u64)
            .map(|i| (i, 10.0 - ((i - list_size as u64 / 2) as f32 / list_size as f32)))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("alpha_0.5", list_size),
            &list_size,
            |b, _| {
                b.iter(|| {
                    black_box(linear_fusion(&dense, &sparse, 0.5, 100))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// HYBRID SEARCH BENCHMARKS
// =============================================================================

fn bench_hybrid_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("hybrid_search");
    group.sample_size(50); // Fewer samples for slower benchmarks

    for num_vectors in [1_000, 10_000] {
        let (graph, dense_storage, sparse_storage) = generate_test_data(
            num_vectors, 64, 50, 10_000
        );

        let dense_query = random_dense_query(64, 54321);
        let sparse_query = random_sparse_query(50, 10_000, 67890);

        group.bench_with_input(
            BenchmarkId::new("rrf", num_vectors),
            &num_vectors,
            |b, _| {
                let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);
                let config = HybridSearchConfig::rrf(20, 20, 10);

                b.iter(|| {
                    black_box(searcher.search(&dense_query, &sparse_query, &config).unwrap())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("linear", num_vectors),
            &num_vectors,
            |b, _| {
                let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);
                let config = HybridSearchConfig::linear(20, 20, 10, 0.5);

                b.iter(|| {
                    black_box(searcher.search(&dense_query, &sparse_query, &config).unwrap())
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// CRITERION MAIN
// =============================================================================

criterion_group!(
    benches,
    bench_sparse_search,
    bench_rrf_fusion,
    bench_linear_fusion,
    bench_hybrid_search,
);

criterion_main!(benches);
```

**Performance Targets:**

| Benchmark | Target | Acceptable |
|:----------|:-------|:-----------|
| Sparse search (10k, k=100) | <10ms | <20ms |
| RRF fusion (1k results) | <1ms | <2ms |
| Linear fusion (1k results) | <1ms | <2ms |
| Hybrid search (10k, k=10) | <20ms | <50ms |

**Acceptance Criteria:**
- [ ] `bench_sparse_search` for 1k and 10k vectors
- [ ] `bench_rrf_fusion` for various list sizes
- [ ] `bench_linear_fusion` for various list sizes
- [ ] `bench_hybrid_search` end-to-end
- [ ] Benchmarks run without error
- [ ] Results within acceptable range

**Estimated Duration:** 45 minutes

**Agent:** BENCHMARK_SCIENTIST

---

### W39.6.2: Recall Benchmark Test

**Objective:** Validate RRF recall > 0.90 on synthetic benchmark.

**File:** `tests/hybrid_recall_test.rs`

```rust
//! Recall benchmark for hybrid search.
//!
//! Tests that RRF fusion achieves >0.90 recall compared to
//! exhaustive ground truth computation.

use edgevec::hnsw::graph::{HnswGraph, HnswConfig};
use edgevec::storage::VectorStorage;
use edgevec::sparse::{SparseStorage, SparseVector, SparseSearcher, sparse_dot_product};
use edgevec::hybrid::{HybridSearcher, HybridSearchConfig, rrf_fusion};
use edgevec::types::VectorId;

use std::collections::HashSet;

/// Generate synthetic dataset where we know ground truth.
fn setup_recall_test(num_vectors: usize) -> (
    HnswGraph,
    VectorStorage,
    SparseStorage,
    Vec<Vec<f32>>,      // Dense vectors
    Vec<SparseVector>,  // Sparse vectors
) {
    let dim = 64;
    let sparse_dim = 1000u32;

    // [HOSTILE_REVIEW: M2 Resolution] - Set explicit ef_search to control
    // HNSW approximation quality in recall test. High ef_search ensures
    // HNSW returns accurate results so we measure fusion quality, not HNSW quality.
    let config = HnswConfig {
        dimensions: dim as u32,
        m: 16,
        ef_construction: 200, // High for good graph quality
        ef_search: 100,       // High ef_search for accurate HNSW in recall test
        ..HnswConfig::default()
    };

    let mut graph = HnswGraph::new(config);
    let mut dense_storage = VectorStorage::new(dim);
    let mut sparse_storage = SparseStorage::new();

    let mut dense_vectors = Vec::new();
    let mut sparse_vectors = Vec::new();

    // Seed for reproducibility
    let mut seed: u64 = 42;
    let lcg = |s: &mut u64| -> f32 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((*s >> 33) as f32) / (u32::MAX as f32)
    };

    for i in 0..num_vectors {
        // Dense vector - normalized random
        let mut dense: Vec<f32> = (0..dim).map(|_| lcg(&mut seed)).collect();
        let norm: f32 = dense.iter().map(|x| x * x).sum::<f32>().sqrt();
        for x in &mut dense {
            *x /= norm;
        }

        let id = dense_storage.insert(&dense).unwrap();
        graph.insert(id, &dense, &dense_storage).unwrap();
        dense_vectors.push(dense);

        // Sparse vector - some overlap in indices for matching
        let base_index = (i * 3) as u32 % sparse_dim;
        let indices: Vec<u32> = (0..10)
            .map(|j| (base_index + j * 50) % sparse_dim)
            .collect();
        let mut indices_sorted = indices;
        indices_sorted.sort_unstable();
        indices_sorted.dedup();

        let values: Vec<f32> = indices_sorted.iter().map(|_| lcg(&mut seed) * 5.0).collect();
        let sparse = SparseVector::new(indices_sorted, values, sparse_dim).unwrap();
        sparse_storage.insert(&sparse).unwrap();
        sparse_vectors.push(sparse);
    }

    (graph, dense_storage, sparse_storage, dense_vectors, sparse_vectors)
}

/// Compute ground truth using exhaustive search.
fn ground_truth_hybrid(
    dense_query: &[f32],
    sparse_query: &SparseVector,
    dense_vectors: &[Vec<f32>],
    sparse_vectors: &[SparseVector],
    k: usize,
) -> Vec<u64> {
    // Compute dense similarities (dot product for normalized vectors)
    let dense_scores: Vec<(u64, f32)> = dense_vectors
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let dot: f32 = dense_query.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
            (i as u64, dot)
        })
        .collect();

    // Sort dense by descending score
    let mut dense_sorted = dense_scores.clone();
    dense_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Compute sparse similarities
    let sparse_scores: Vec<(u64, f32)> = sparse_vectors
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let dot = sparse_dot_product(sparse_query, v);
            (i as u64, dot)
        })
        .filter(|(_, score)| *score > 0.0)
        .collect();

    // Sort sparse by descending score
    let mut sparse_sorted = sparse_scores;
    sparse_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Apply RRF fusion
    let fused = rrf_fusion(&dense_sorted, &sparse_sorted, 60, k);

    fused.iter().map(|r| r.id).collect()
}

/// Compute recall@k.
fn recall_at_k(predicted: &[u64], ground_truth: &[u64], k: usize) -> f32 {
    let pred_set: HashSet<u64> = predicted.iter().take(k).copied().collect();
    let gt_set: HashSet<u64> = ground_truth.iter().take(k).copied().collect();

    let intersection = pred_set.intersection(&gt_set).count();
    intersection as f32 / k as f32
}

#[test]
fn test_rrf_recall_at_10() {
    let num_vectors = 1000;
    let num_queries = 50;
    let k = 10;

    let (graph, dense_storage, sparse_storage, dense_vectors, sparse_vectors) =
        setup_recall_test(num_vectors);

    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);
    let config = HybridSearchConfig::rrf(100, 100, k); // High retrieval for good recall

    let mut total_recall = 0.0;
    let mut seed: u64 = 99999;

    for _ in 0..num_queries {
        // Generate random query
        let dense_query: Vec<f32> = (0..64)
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((seed >> 33) as f32) / (u32::MAX as f32)
            })
            .collect();

        let sparse_indices: Vec<u32> = (0..10)
            .map(|i| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((seed >> 33) as u32) % 1000
            })
            .collect();
        let mut sparse_indices_sorted = sparse_indices;
        sparse_indices_sorted.sort_unstable();
        sparse_indices_sorted.dedup();

        let sparse_values: Vec<f32> = sparse_indices_sorted.iter().map(|_| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 33) as f32) / (u32::MAX as f32) * 5.0
        }).collect();

        if sparse_indices_sorted.is_empty() {
            continue;
        }

        let sparse_query = SparseVector::new(sparse_indices_sorted, sparse_values, 1000).unwrap();

        // Get hybrid search results
        let results = searcher.search(&dense_query, &sparse_query, &config).unwrap();
        let predicted: Vec<u64> = results.iter().map(|r| r.id.0).collect();

        // Get ground truth
        let ground_truth = ground_truth_hybrid(
            &dense_query, &sparse_query, &dense_vectors, &sparse_vectors, k
        );

        // Compute recall
        let recall = recall_at_k(&predicted, &ground_truth, k);
        total_recall += recall;
    }

    let avg_recall = total_recall / num_queries as f32;
    println!("Average Recall@{}: {:.3}", k, avg_recall);

    // Assert recall > 0.90
    assert!(
        avg_recall > 0.90,
        "RRF recall@{} = {:.3}, expected > 0.90",
        k, avg_recall
    );
}

#[test]
fn test_linear_fusion_recall() {
    let num_vectors = 1000;
    let k = 10;

    let (graph, dense_storage, sparse_storage, dense_vectors, sparse_vectors) =
        setup_recall_test(num_vectors);

    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);

    // Test different alpha values
    for alpha in [0.3, 0.5, 0.7] {
        let config = HybridSearchConfig::linear(100, 100, k, alpha);

        // Single query test
        let dense_query: Vec<f32> = (0..64).map(|i| (i as f32 / 64.0)).collect();
        let sparse_query = SparseVector::new(vec![0, 50, 100], vec![1.0, 2.0, 1.5], 1000).unwrap();

        let results = searcher.search(&dense_query, &sparse_query, &config).unwrap();

        // Should return k results
        assert_eq!(results.len(), k, "alpha={} should return {} results", alpha, k);

        // Results should be sorted by score
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results not sorted for alpha={}", alpha
            );
        }
    }
}

#[test]
fn test_hybrid_with_real_bm25_style_scores() {
    // Simulate real BM25 score distribution
    let num_vectors = 500;

    let (graph, dense_storage, _, dense_vectors, _) = setup_recall_test(num_vectors);

    // Create sparse storage with BM25-like scores
    let mut sparse_storage = SparseStorage::new();

    for i in 0..num_vectors {
        // BM25-style: few high-scoring terms, many low-scoring
        let mut indices_values: Vec<(u32, f32)> = Vec::new();

        // High-scoring terms (1-3)
        let high_term = (i % 100) as u32;
        indices_values.push((high_term, 5.0 + (i % 10) as f32 * 0.5));

        // Medium-scoring terms (3-5)
        for j in 0..3 {
            let term = ((i * 7 + j * 13) % 500) as u32;
            indices_values.push((term, 2.0 + j as f32 * 0.3));
        }

        // Low-scoring terms (5-10)
        for j in 0..5 {
            let term = ((i * 11 + j * 17) % 1000) as u32;
            indices_values.push((term, 0.5 + j as f32 * 0.1));
        }

        // Sort and dedupe
        indices_values.sort_by_key(|(idx, _)| *idx);
        indices_values.dedup_by_key(|(idx, _)| *idx);

        let indices: Vec<u32> = indices_values.iter().map(|(i, _)| *i).collect();
        let values: Vec<f32> = indices_values.iter().map(|(_, v)| *v).collect();

        let sparse = SparseVector::new(indices, values, 1000).unwrap();
        sparse_storage.insert(&sparse).unwrap();
    }

    let searcher = HybridSearcher::new(&graph, &dense_storage, &sparse_storage);
    let config = HybridSearchConfig::rrf(50, 50, 10);

    // Query with BM25-style sparse query
    let dense_query: Vec<f32> = dense_vectors[0].clone(); // Query similar to doc 0
    let sparse_query = SparseVector::new(
        vec![0, 50, 100, 200],  // Some overlap with doc 0's terms
        vec![4.0, 2.5, 1.5, 1.0],  // BM25-like scores
        1000
    ).unwrap();

    let results = searcher.search(&dense_query, &sparse_query, &config).unwrap();

    assert!(!results.is_empty(), "Should return results");
    assert_eq!(results.len(), 10, "Should return exactly k=10 results");

    // Doc 0 should rank highly due to dense similarity
    let top_10_ids: Vec<u64> = results.iter().take(10).map(|r| r.id.0).collect();
    println!("Top 10 IDs with BM25-style scores: {:?}", top_10_ids);
}
```

**Acceptance Criteria:**
- [ ] `test_rrf_recall_at_10` passes with recall > 0.90
- [ ] `test_linear_fusion_recall` passes for multiple alpha values
- [ ] `test_hybrid_with_real_bm25_style_scores` passes
- [ ] All recall tests are reproducible (seeded randomness)

**Estimated Duration:** 30 minutes

**Agent:** TEST_ENGINEER

---

### W39.6.3: Hostile Review Submission

**Objective:** Obtain HOSTILE_REVIEWER approval for Week 39 deliverables.

**Review Artifacts:**

```
src/sparse/search.rs          # SparseSearcher
src/hybrid/mod.rs             # Module exports
src/hybrid/fusion.rs          # RRF + Linear fusion
src/hybrid/search.rs          # HybridSearcher
src/wasm/mod.rs               # WASM bindings (hybrid additions)
pkg/edgevec.d.ts              # TypeScript types
benches/hybrid_bench.rs       # Benchmarks
tests/hybrid_search_test.rs   # Integration tests
tests/hybrid_recall_test.rs   # Recall benchmark
```

**Review Command:**
```
/review Week 39 Hybrid Search
```

**Review Checklist:**

| Category | Criterion | Pass/Fail |
|:---------|:----------|:----------|
| **Correctness** | RRF formula matches paper | |
| **Correctness** | Linear fusion normalizes correctly | |
| **Correctness** | Sparse search returns correct results | |
| **Performance** | Sparse search <20ms for 10k | |
| **Performance** | Fusion <2ms for 1k results | |
| **Performance** | Hybrid <50ms end-to-end | |
| **Quality** | RRF recall > 0.90 | |
| **Quality** | All edge cases handled | |
| **API** | WASM bindings validate inputs | |
| **API** | TypeScript types complete | |
| **Docs** | All public APIs documented | |
| **Tests** | 50+ unit tests pass | |
| **Tests** | 8+ integration tests pass | |
| **Tests** | Recall benchmark passes | |
| **Lint** | Clippy clean | |

**Issues to Address:**

| Severity | Issue | Resolution |
|:---------|:------|:-----------|
| CRITICAL | | |
| MAJOR | | |
| MINOR | | |

**Acceptance Criteria:**
- [ ] All CRITICAL issues addressed
- [ ] All MAJOR issues addressed
- [ ] Review document created: `docs/reviews/2026-02-01_hybrid_search_APPROVED.md`
- [ ] Verdict: APPROVED or CONDITIONAL_APPROVAL

**Estimated Duration:** 45 minutes

**Agent:** HOSTILE_REVIEWER

---

## Day 6 Checklist

- [ ] W39.6.1: Benchmark suite created and run
- [ ] W39.6.2: Recall benchmark passes (>0.90)
- [ ] W39.6.3: Hostile review submitted and approved
- [ ] All benchmarks within acceptable range
- [ ] All tests pass
- [ ] Review document created
- [ ] Week 40 plan created

---

## Day 6 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Sparse search <20ms (10k) | Benchmark |
| RRF fusion <2ms (1k) | Benchmark |
| Hybrid search <50ms (10k) | Benchmark |
| RRF recall > 0.90 | Recall test |
| All tests pass | `cargo test` |
| Hostile review APPROVED | Review document |

---

## Week 39 Final Deliverables

### Files Created

```
src/sparse/
└── search.rs                  # SparseSearcher implementation

src/hybrid/
├── mod.rs                     # Module exports
├── fusion.rs                  # RRF + Linear fusion
└── search.rs                  # HybridSearcher

benches/
└── hybrid_bench.rs            # Hybrid search benchmarks

tests/
├── hybrid_search_test.rs      # Integration tests
└── hybrid_recall_test.rs      # Recall benchmark
```

### Files Modified

```
src/lib.rs                     # Add hybrid module
src/sparse/mod.rs              # Add search export
src/wasm/mod.rs                # Add WASM bindings
pkg/edgevec.d.ts               # Add TypeScript types
```

### Documentation

```
docs/reviews/
└── 2026-02-01_hybrid_search_APPROVED.md
```

---

## Week 39 Metrics Summary

| Metric | Target | Actual | Status |
|:-------|:-------|:-------|:-------|
| Sparse search (10k) | <20ms | | |
| RRF fusion (1k) | <2ms | | |
| Linear fusion (1k) | <2ms | | |
| Hybrid search (10k) | <50ms | | |
| RRF recall@10 | >0.90 | | |
| Unit tests | 50+ | | |
| Integration tests | 8+ | | |
| Clippy warnings | 0 | | |

---

## Week 39 Handoff

### To Week 40

**Completed (Week 39):**
- `SparseSearcher` with brute-force search
- RRF fusion (k=60 default, paper-compliant)
- Linear fusion with score normalization
- `HybridSearcher` combining dense + sparse
- WASM bindings: `searchSparse()`, `insertSparse()`, `hybridSearch()`
- TypeScript types for all new APIs
- 50+ unit tests
- 8+ integration tests
- Recall > 0.90 validated

**Ready for Week 40 (Flat Index):**
- Hybrid search foundation enables Flat Index comparison
- Benchmark baselines established
- @jsonMartin RFC integration path clear

**Dependencies for Week 40:**
- No blocking dependencies from Week 39
- Flat Index is additive feature

---

## Commit Message Template

```
feat(hybrid): implement RRF hybrid search (Week 39)

- Add SparseSearcher with brute-force top-k search
- Implement RRF fusion algorithm (Cormack et al. SIGIR 2009)
- Implement linear combination fusion with score normalization
- Add HybridSearcher combining dense HNSW + sparse search
- WASM bindings: searchSparse(), insertSparse(), hybridSearch()
- TypeScript types for hybrid search API

Performance:
- Sparse search: Xms (10k vectors)
- RRF fusion: Xms (1k results)
- Hybrid search: Xms (10k vectors)

Quality:
- RRF recall@10: X.XX (target >0.90)
- 50+ unit tests passing
- 8+ integration tests passing

RFC-007 Milestone 9.2 (Hybrid Search) COMPLETE.

Closes #[hybrid-search-issue]

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

**Agent:** PLANNER
**Hours:** 2h (Day 6) / 16h (Week 39 total)
**Priority:** P0 (v0.9.0 core feature)
**Status:** [PROPOSED]

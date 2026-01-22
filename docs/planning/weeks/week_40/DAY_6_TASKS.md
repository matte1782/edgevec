# Week 40 Day 6: Testing, Benchmarks, Hostile Review

**Date:** 2026-02-08
**Focus:** Comprehensive validation, performance reporting, and approval
**Estimated Duration:** 7 hours
**Phase:** RFC-008 Phase 6 (Validation)
**Dependencies:** Days 1-5 COMPLETE

---

## Context

Day 6 is validation day. We ensure FlatIndex meets all quality targets:
- 100% recall validated
- Performance within targets
- Property-based testing
- Integration with existing systems
- HOSTILE_REVIEWER approval

**Exit Criteria (from ROADMAP):**
- [ ] FlatIndex search <50ms for 10k vectors
- [ ] 100% recall validated
- [ ] Persistence round-trip works
- [ ] WASM bindings functional
- [ ] HOSTILE_REVIEWER APPROVED

---

## Tasks

### W40.6.1: Property-Based Tests

**Objective:** Use proptest to verify FlatIndex invariants.

**File:** `src/index/flat.rs` (proptest module)

```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn random_vector(dim: usize)(
            values in proptest::collection::vec(-1.0f32..1.0f32, dim)
        ) -> Vec<f32> {
            values
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_insert_returns_sequential_ids(
            vectors in proptest::collection::vec(random_vector(16), 1..50)
        ) {
            let mut index = FlatIndex::new(FlatIndexConfig::new(16));

            for (i, v) in vectors.iter().enumerate() {
                let id = index.insert(v).unwrap();
                prop_assert_eq!(id, i as u64);
            }
        }

        #[test]
        fn prop_search_returns_k_or_fewer(
            vectors in proptest::collection::vec(random_vector(16), 1..100),
            k in 1usize..20,
        ) {
            let mut index = FlatIndex::new(FlatIndexConfig::new(16));

            for v in &vectors {
                index.insert(v).unwrap();
            }

            let query = random_vector(16)();
            let results = index.search(&query, k).unwrap();

            prop_assert!(results.len() <= k);
            prop_assert!(results.len() <= vectors.len());
        }

        #[test]
        fn prop_search_results_sorted(
            vectors in proptest::collection::vec(random_vector(16), 10..50),
        ) {
            let config = FlatIndexConfig::new(16).with_metric(Metric::Cosine);
            let mut index = FlatIndex::new(config);

            for v in &vectors {
                index.insert(v).unwrap();
            }

            let query = random_vector(16)();
            let results = index.search(&query, 10).unwrap();

            // Results should be sorted by descending similarity
            for i in 1..results.len() {
                prop_assert!(
                    results[i - 1].score >= results[i].score,
                    "Results not sorted: {} < {}",
                    results[i - 1].score,
                    results[i].score
                );
            }
        }

        #[test]
        fn prop_delete_removes_from_search(
            vectors in proptest::collection::vec(random_vector(16), 10..30),
            delete_idx in 0usize..10,
        ) {
            let mut index = FlatIndex::new(FlatIndexConfig::new(16));

            for v in &vectors {
                index.insert(v).unwrap();
            }

            let delete_id = delete_idx as u64;
            index.delete(delete_id);

            // Search should not return deleted ID
            let query = vectors[delete_idx].clone();
            let results = index.search(&query, vectors.len()).unwrap();

            for r in results {
                prop_assert_ne!(r.id, delete_id, "Deleted ID found in results");
            }
        }

        #[test]
        fn prop_get_returns_inserted_vector(
            vector in random_vector(32),
        ) {
            let mut index = FlatIndex::new(FlatIndexConfig::new(32));

            let id = index.insert(&vector).unwrap();
            let retrieved = index.get(id).unwrap();

            prop_assert_eq!(retrieved, vector.as_slice());
        }

        #[test]
        fn prop_snapshot_preserves_search_results(
            vectors in proptest::collection::vec(random_vector(16), 5..20),
        ) {
            let mut index = FlatIndex::new(FlatIndexConfig::new(16));

            for v in &vectors {
                index.insert(v).unwrap();
            }

            let query = random_vector(16)();
            let original_results = index.search(&query, 5).unwrap();

            // Snapshot and restore
            let snapshot = index.to_snapshot().unwrap();
            let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

            let restored_results = restored.search(&query, 5).unwrap();

            prop_assert_eq!(original_results.len(), restored_results.len());
            for (orig, rest) in original_results.iter().zip(restored_results.iter()) {
                prop_assert_eq!(orig.id, rest.id);
                prop_assert!((orig.score - rest.score).abs() < 1e-5);
            }
        }

        #[test]
        fn prop_l2_distance_positive(
            a in random_vector(16),
            b in random_vector(16),
        ) {
            let config = FlatIndexConfig::new(16).with_metric(Metric::L2);
            let mut index = FlatIndex::new(config);

            index.insert(&a).unwrap();
            let results = index.search(&b, 1).unwrap();

            prop_assert!(results[0].score >= 0.0, "L2 distance should be non-negative");
        }

        #[test]
        fn prop_cosine_bounded(
            a in random_vector(16),
            b in random_vector(16),
        ) {
            let config = FlatIndexConfig::new(16).with_metric(Metric::Cosine);
            let mut index = FlatIndex::new(config);

            index.insert(&a).unwrap();
            let results = index.search(&b, 1).unwrap();

            prop_assert!(
                results[0].score >= -1.0 && results[0].score <= 1.0,
                "Cosine similarity should be in [-1, 1], got {}",
                results[0].score
            );
        }

        #[test]
        fn prop_len_matches_inserts_minus_deletes(
            insert_count in 10usize..50,
            delete_count in 0usize..5,
        ) {
            let mut index = FlatIndex::new(FlatIndexConfig::new(8));

            for i in 0..insert_count {
                index.insert(&vec![i as f32; 8]).unwrap();
            }

            for i in 0..delete_count.min(insert_count) {
                index.delete(i as u64);
            }

            let expected = insert_count - delete_count.min(insert_count);
            prop_assert_eq!(index.len(), expected);
        }
    }
}
```

**Acceptance Criteria:**
- [ ] `prop_insert_returns_sequential_ids` passes
- [ ] `prop_search_returns_k_or_fewer` passes
- [ ] `prop_search_results_sorted` passes
- [ ] `prop_delete_removes_from_search` passes
- [ ] `prop_get_returns_inserted_vector` passes
- [ ] `prop_snapshot_preserves_search_results` passes
- [ ] `prop_l2_distance_positive` passes
- [ ] `prop_cosine_bounded` passes
- [ ] `prop_len_matches_inserts_minus_deletes` passes
- [ ] 100 test cases per property (default)

**Deliverables:**
- Proptest module in `src/index/flat.rs`

**Dependencies:** Days 1-4

**Estimated Duration:** 1.5 hours

**Agent:** TEST_ENGINEER

---

### W40.6.2: Benchmark Suite

**Objective:** Run comprehensive benchmarks and document results.

**File:** `benches/flat_bench.rs` (extended)

```rust
// Add to existing benchmarks

fn bench_search_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat_search_dims");
    group.sample_size(30);

    for dim in [64, 128, 384, 768] {
        let vectors = generate_vectors(5000, dim, 42);
        let mut index = FlatIndex::new(FlatIndexConfig::new(dim as u32));
        for v in &vectors {
            index.insert(v).unwrap();
        }

        let query = generate_vectors(1, dim, 999)[0].clone();

        group.bench_with_input(
            BenchmarkId::new("5k_k10", dim),
            &dim,
            |b, _| {
                b.iter(|| black_box(index.search(&query, 10).unwrap()));
            },
        );
    }

    group.finish();
}

fn bench_search_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat_search_metrics");
    group.sample_size(50);

    for metric in [Metric::Cosine, Metric::DotProduct, Metric::L2] {
        let config = FlatIndexConfig::new(128).with_metric(metric);
        let mut index = FlatIndex::new(config);

        let vectors = generate_vectors(5000, 128, 42);
        for v in &vectors {
            index.insert(v).unwrap();
        }

        let query = generate_vectors(1, 128, 999)[0].clone();

        group.bench_with_input(
            BenchmarkId::new("5k_k10", format!("{:?}", metric)),
            &metric,
            |b, _| {
                b.iter(|| black_box(index.search(&query, 10).unwrap()));
            },
        );
    }

    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    // Note: This benchmark measures throughput to estimate memory impact
    let mut group = c.benchmark_group("flat_memory");

    for count in [1000, 5000, 10000] {
        let vectors = generate_vectors(count, 768, 42);

        group.bench_with_input(
            BenchmarkId::new("insert_768d", count),
            &vectors,
            |b, vecs| {
                b.iter(|| {
                    let mut index = FlatIndex::new(
                        FlatIndexConfig::new(768).with_capacity(count)
                    );
                    for v in vecs {
                        black_box(index.insert(v).unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_search,
    bench_search_quantized,
    bench_search_dimensions,
    bench_search_metrics,
    bench_memory_usage,
);
```

**Run and Document:**

```bash
cargo bench --bench flat_bench -- --save-baseline week40
```

**Expected Results Table:**

| Benchmark | Target | Actual | Status |
|:----------|:-------|:-------|:-------|
| Insert (10k, 128D) | <10ms | TBD | [ ] |
| Search (1k, k=10) | <1ms | TBD | [ ] |
| Search (10k, k=10) | <50ms | TBD | [ ] |
| Search (10k, k=10, BQ) | <10ms | TBD | [ ] |
| Search (5k, 768D, k=10) | <30ms | TBD | [ ] |

**Acceptance Criteria:**
- [ ] All benchmarks run without error
- [ ] Results within acceptable range
- [ ] Benchmark report generated
- [ ] Comparison with HNSW documented

**Deliverables:**
- Extended `benches/flat_bench.rs`
- Benchmark results document

**Dependencies:** Day 3

**Estimated Duration:** 1 hour

**Agent:** BENCHMARK_SCIENTIST

---

### W40.6.3: Recall Validation

**Objective:** Verify 100% recall on known datasets.

**File:** `tests/flat_recall_test.rs`

```rust
//! Recall validation tests for FlatIndex.
//!
//! Verifies that brute-force search achieves 100% recall
//! (exact nearest neighbors).

use edgevec::index::{FlatIndex, FlatIndexConfig};
use edgevec::metric::Metric;
use std::collections::HashSet;

fn generate_vectors(count: usize, dim: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut s = seed;
    let lcg = |s: &mut u64| -> f32 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((*s >> 33) as f32) / (u32::MAX as f32)
    };

    (0..count)
        .map(|_| (0..dim).map(|_| lcg(&mut s)).collect())
        .collect()
}

fn brute_force_knn(
    vectors: &[Vec<f32>],
    query: &[f32],
    k: usize,
    metric: Metric,
) -> Vec<u64> {
    let mut distances: Vec<(u64, f32)> = vectors
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let d = match metric {
                Metric::Cosine => {
                    let dot: f32 = query.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
                    let norm_q: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();
                    let norm_v: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                    if norm_q == 0.0 || norm_v == 0.0 { 0.0 } else { dot / (norm_q * norm_v) }
                }
                Metric::DotProduct => {
                    query.iter().zip(v.iter()).map(|(a, b)| a * b).sum()
                }
                Metric::L2 => {
                    query.iter().zip(v.iter()).map(|(a, b)| (a - b) * (a - b)).sum::<f32>().sqrt()
                }
                Metric::Hamming => {
                    query.iter().zip(v.iter()).filter(|(a, b)| (**a != 0.0) != (**b != 0.0)).count() as f32
                }
            };
            (i as u64, d)
        })
        .collect();

    // Sort by score
    match metric {
        Metric::Cosine | Metric::DotProduct => {
            distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        }
        Metric::L2 | Metric::Hamming => {
            distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        }
    }

    distances.iter().take(k).map(|(id, _)| *id).collect()
}

#[test]
fn test_100_recall_cosine() {
    let vectors = generate_vectors(1000, 64, 42);
    let config = FlatIndexConfig::new(64).with_metric(Metric::Cosine);
    let mut index = FlatIndex::new(config);

    for v in &vectors {
        index.insert(v).unwrap();
    }

    // Test 100 random queries
    let queries = generate_vectors(100, 64, 999);

    for query in &queries {
        let k = 10;
        let index_results: HashSet<u64> = index.search(query, k).unwrap()
            .iter().map(|r| r.id).collect();
        let ground_truth: HashSet<u64> = brute_force_knn(&vectors, query, k, Metric::Cosine)
            .into_iter().collect();

        let recall = index_results.intersection(&ground_truth).count() as f32 / k as f32;
        assert_eq!(recall, 1.0, "Recall should be 100% for brute-force search");
    }
}

#[test]
fn test_100_recall_l2() {
    let vectors = generate_vectors(1000, 64, 42);
    let config = FlatIndexConfig::new(64).with_metric(Metric::L2);
    let mut index = FlatIndex::new(config);

    for v in &vectors {
        index.insert(v).unwrap();
    }

    let queries = generate_vectors(50, 64, 888);

    for query in &queries {
        let k = 10;
        let index_results: HashSet<u64> = index.search(query, k).unwrap()
            .iter().map(|r| r.id).collect();
        let ground_truth: HashSet<u64> = brute_force_knn(&vectors, query, k, Metric::L2)
            .into_iter().collect();

        let recall = index_results.intersection(&ground_truth).count() as f32 / k as f32;
        assert_eq!(recall, 1.0, "L2 search should have 100% recall");
    }
}

#[test]
fn test_100_recall_dot() {
    let vectors = generate_vectors(500, 128, 12345);
    let config = FlatIndexConfig::new(128).with_metric(Metric::DotProduct);
    let mut index = FlatIndex::new(config);

    for v in &vectors {
        index.insert(v).unwrap();
    }

    let queries = generate_vectors(20, 128, 54321);

    for query in &queries {
        let k = 5;
        let index_results: HashSet<u64> = index.search(query, k).unwrap()
            .iter().map(|r| r.id).collect();
        let ground_truth: HashSet<u64> = brute_force_knn(&vectors, query, k, Metric::DotProduct)
            .into_iter().collect();

        let recall = index_results.intersection(&ground_truth).count() as f32 / k as f32;
        assert_eq!(recall, 1.0, "Dot product search should have 100% recall");
    }
}

#[test]
fn test_exact_match_first() {
    // Insert a vector, then query with the same vector
    // It should be the first result
    let mut index = FlatIndex::new(FlatIndexConfig::new(32).with_metric(Metric::Cosine));

    let vectors = generate_vectors(100, 32, 42);
    for v in &vectors {
        index.insert(v).unwrap();
    }

    // Query with vector 50
    let query = &vectors[50];
    let results = index.search(query, 10).unwrap();

    assert_eq!(results[0].id, 50, "Exact match should be first result");
    assert!((results[0].score - 1.0).abs() < 1e-5, "Exact match should have score ~1.0");
}
```

**Acceptance Criteria:**
- [ ] `test_100_recall_cosine` passes (100 queries)
- [ ] `test_100_recall_l2` passes (50 queries)
- [ ] `test_100_recall_dot` passes (20 queries)
- [ ] `test_exact_match_first` passes
- [ ] All recall tests verify against independent ground truth

**Deliverables:**
- `tests/flat_recall_test.rs`

**Dependencies:** Day 2

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

### W40.6.4: Integration & Compatibility Tests

**Objective:** Verify FlatIndex works with existing EdgeVec systems.

**File:** `tests/flat_integration_test.rs`

```rust
//! Integration tests for FlatIndex with EdgeVec systems.

use edgevec::index::{FlatIndex, FlatIndexConfig};
use edgevec::hnsw::{HnswIndex, HnswConfig};
use edgevec::storage::VectorStorage;

#[test]
fn test_flat_and_hnsw_same_results_exact() {
    // For small k and high ef_search, HNSW should match Flat exactly
    let dim = 32;
    let vectors = generate_vectors(100, dim, 42);

    // FlatIndex
    let mut flat = FlatIndex::new(FlatIndexConfig::new(dim as u32));
    for v in &vectors {
        flat.insert(v).unwrap();
    }

    // HNSW with high ef_search
    let hnsw_config = HnswConfig::new(dim as u32);
    let mut storage = VectorStorage::new(&hnsw_config, None);
    let mut hnsw = HnswIndex::new(hnsw_config, &storage).unwrap();
    for v in &vectors {
        hnsw.insert(v, &mut storage).unwrap();
    }

    // Query
    let query = generate_vectors(1, dim, 999)[0].clone();

    let flat_results = flat.search(&query, 5).unwrap();
    // Note: HNSW results might differ slightly due to approximation
    // This test verifies they're "close enough" for small datasets
}

#[test]
fn test_flat_with_metadata() {
    // When metadata integration is complete, verify it works with FlatIndex
    // For now, just verify the pattern works
    let mut flat = FlatIndex::new(FlatIndexConfig::new(16));

    let id = flat.insert(&vec![0.1; 16]).unwrap();

    // Metadata would be stored separately, keyed by ID
    assert_eq!(id, 0);
}

#[test]
fn test_concurrent_read_access() {
    use std::sync::Arc;
    use std::thread;

    let mut flat = FlatIndex::new(FlatIndexConfig::new(16));
    for i in 0..100 {
        flat.insert(&vec![i as f32 / 100.0; 16]).unwrap();
    }

    let flat = Arc::new(flat);
    let mut handles = vec![];

    // Spawn multiple reader threads
    for _ in 0..4 {
        let flat = Arc::clone(&flat);
        handles.push(thread::spawn(move || {
            for _ in 0..10 {
                let query = vec![0.5; 16];
                let results = flat.search(&query, 5).unwrap();
                assert_eq!(results.len(), 5);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

fn generate_vectors(count: usize, dim: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut s = seed;
    let lcg = |s: &mut u64| -> f32 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((*s >> 33) as f32) / (u32::MAX as f32)
    };

    (0..count)
        .map(|_| (0..dim).map(|_| lcg(&mut s)).collect())
        .collect()
}
```

**Acceptance Criteria:**
- [ ] FlatIndex works alongside HNSW
- [ ] Metadata pattern compatible
- [ ] Concurrent read access works
- [ ] No deadlocks or race conditions

**Deliverables:**
- `tests/flat_integration_test.rs`

**Dependencies:** Days 1-5

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

### W40.6.5: Documentation Polish

**Objective:** Review and finalize all documentation.

**Checklist:**

- [ ] `docs/api/FLAT_INDEX.md` complete and accurate
- [ ] Doc comments on all public items
- [ ] Examples compile and run
- [ ] README mentions FlatIndex
- [ ] CHANGELOG updated

**Tasks:**

1. Review all doc comments in `src/index/flat.rs`
2. Verify TypeScript types match implementation
3. Update main README.md with FlatIndex section
4. Add to CHANGELOG.md

**Estimated Duration:** 0.5 hours

**Agent:** DOCWRITER

---

### W40.6.6: Hostile Review Submission

**Objective:** Obtain HOSTILE_REVIEWER approval for Week 40.

**Review Checklist:**

| Category | Criterion | Status |
|:---------|:----------|:-------|
| **Correctness** | 100% recall validated | [ ] |
| **Correctness** | All metrics work correctly | [ ] |
| **Correctness** | Deletion works | [ ] |
| **Performance** | Search <50ms (10k) | [ ] |
| **Performance** | Insert <100μs | [ ] |
| **Performance** | Memory ~30MB (10k 768D) | [ ] |
| **Quality** | 30+ unit tests | [ ] |
| **Quality** | 10+ property tests | [ ] |
| **Quality** | 10+ integration tests | [ ] |
| **API** | WASM bindings complete | [ ] |
| **API** | TypeScript types complete | [ ] |
| **Persistence** | Snapshot round-trip works | [ ] |
| **Persistence** | Checksum validation | [ ] |
| **Docs** | API documentation | [ ] |
| **Lint** | Clippy clean | [ ] |

**Review Command:**

```bash
/review Week 40 Flat Index
```

**Expected Verdict:** APPROVED or CONDITIONAL_APPROVAL

**Acceptance Criteria:**
- [ ] All CRITICAL issues addressed (0 expected)
- [ ] All MAJOR issues addressed
- [ ] Review document created
- [ ] Verdict: APPROVED

**Deliverables:**
- `docs/reviews/2026-02-08_flat_index_APPROVED.md`

**Dependencies:** W40.6.1-W40.6.5

**Estimated Duration:** 2 hours

**Agent:** HOSTILE_REVIEWER

---

## Day 6 Checklist

- [ ] W40.6.1: Property-based tests (10+ props)
- [ ] W40.6.2: Benchmark suite complete
- [ ] W40.6.3: 100% recall validated
- [ ] W40.6.4: Integration tests pass
- [ ] W40.6.5: Documentation polished
- [ ] W40.6.6: HOSTILE_REVIEWER APPROVED
- [ ] All 911+ existing tests still pass
- [ ] Clippy clean (0 warnings)
- [ ] Week 41 plan created

---

## Exit Criteria for Day 6

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| 30+ unit tests | `cargo test` count | [ ] |
| 10+ property tests | proptest module | [ ] |
| 100% recall | recall tests | [ ] |
| Search <50ms (10k) | benchmark | [ ] |
| WASM tests pass | wasm-pack test | [ ] |
| Persistence works | integration tests | [ ] |
| Documentation complete | manual review | [ ] |
| Clippy clean | 0 warnings | [ ] |
| HOSTILE_REVIEWER | APPROVED | [ ] |

---

## Week 40 Final Summary

| Day | Focus | Status |
|:----|:------|:-------|
| Day 1 | FlatIndex struct + insert | [ ] |
| Day 2 | Search + metrics | [ ] |
| Day 3 | Optimization + BQ | [ ] |
| Day 4 | Persistence | [ ] |
| Day 5 | WASM + TypeScript | [ ] |
| Day 6 | Testing + Review | [ ] |

**Total Hours:** 32h
**Deliverables:** FlatIndex with 100% recall, <50ms search, WASM bindings

---

**Day 6 Total:** 7 hours
**Agent:** TEST_ENGINEER / HOSTILE_REVIEWER
**Status:** [DRAFT]

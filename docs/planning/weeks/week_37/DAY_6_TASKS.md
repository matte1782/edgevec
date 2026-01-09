# Week 37 Day 6: Benchmarks + Hostile Review

**Date:** 2026-01-17
**Focus:** Validate performance targets and get hostile review
**Estimated Duration:** 2 hours
**Phase:** RFC-007 Implementation Phase 1 (Core Types)
**Dependencies:** Days 1-5 — ALL MUST BE COMPLETE

---

## Tasks

### W37.6.1: Create Benchmark File

**Objective:** Create `benches/sparse_bench.rs` with criterion benchmarks.

**Rust Implementation:**

```rust
// benches/sparse_bench.rs

//! Benchmarks for sparse vector operations.
//!
//! Performance targets from RFC-007:
//! - Dot product (50 nnz): P50 <300ns, P99 <500ns
//! - Dot product (100 nnz): P50 <600ns, P99 <1μs

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use edgevec::sparse::{SparseVector, sparse_dot_product, sparse_cosine, sparse_norm};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

/// Generate a random sparse vector with given parameters.
///
/// Uses deterministic RNG for reproducible benchmarks.
fn random_sparse(dim: u32, nnz: usize, seed: u64) -> SparseVector {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // Generate unique sorted indices
    let mut indices: Vec<u32> = (0..dim).collect();
    indices.shuffle(&mut rng);
    indices.truncate(nnz);
    indices.sort();

    // Generate random values
    let values: Vec<f32> = (0..nnz)
        .map(|_| rng.gen_range(-1.0..1.0))
        .collect();

    SparseVector::new(indices, values, dim)
        .expect("Generated vector should be valid")
}

/// Benchmark dot product at various sparsity levels.
fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_dot_product");

    // Configure for accurate P99 measurement
    group.sample_size(1000);
    group.measurement_time(std::time::Duration::from_secs(10));

    let dim = 10_000u32;

    for nnz in [10, 50, 100, 200, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("nnz_{}", nnz)),
            &nnz,
            |b, &nnz| {
                let a = random_sparse(dim, nnz, 42);
                let query = random_sparse(dim, nnz, 123);

                b.iter(|| {
                    sparse_dot_product(black_box(&a), black_box(&query))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark dot product with varying overlap.
fn bench_dot_product_overlap(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_dot_overlap");
    group.sample_size(500);

    let dim = 10_000u32;
    let nnz = 100usize;

    // No overlap (different index ranges)
    group.bench_function("no_overlap", |b| {
        let a = SparseVector::new(
            (0..nnz as u32).collect(),
            vec![1.0; nnz],
            dim
        ).unwrap();
        let query = SparseVector::new(
            (5000..(5000 + nnz as u32)).collect(),
            vec![1.0; nnz],
            dim
        ).unwrap();

        b.iter(|| sparse_dot_product(black_box(&a), black_box(&query)));
    });

    // Full overlap (same indices)
    group.bench_function("full_overlap", |b| {
        let a = random_sparse(dim, nnz, 42);
        let query = SparseVector::new(
            a.indices().to_vec(),
            vec![1.0; nnz],
            dim
        ).unwrap();

        b.iter(|| sparse_dot_product(black_box(&a), black_box(&query)));
    });

    // Partial overlap (~50%)
    group.bench_function("partial_overlap", |b| {
        let a = random_sparse(dim, nnz, 42);
        let query = random_sparse(dim, nnz, 43); // Different seed, ~50% overlap expected

        b.iter(|| sparse_dot_product(black_box(&a), black_box(&query)));
    });

    group.finish();
}

/// Benchmark cosine similarity.
fn bench_cosine(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_cosine");
    group.sample_size(500);

    let dim = 10_000u32;

    for nnz in [50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("nnz_{}", nnz)),
            &nnz,
            |b, &nnz| {
                let a = random_sparse(dim, nnz, 42);
                let query = random_sparse(dim, nnz, 123);

                b.iter(|| {
                    sparse_cosine(black_box(&a), black_box(&query))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark norm calculation.
fn bench_norm(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_norm");
    group.sample_size(500);

    let dim = 10_000u32;

    for nnz in [50, 100, 200, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("nnz_{}", nnz)),
            &nnz,
            |b, &nnz| {
                let v = random_sparse(dim, nnz, 42);

                b.iter(|| {
                    sparse_norm(black_box(&v))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark vector construction.
fn bench_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_construction");
    group.sample_size(500);

    let dim = 10_000u32;

    // Benchmark new() with pre-sorted data
    for nnz in [50, 100, 200] {
        let indices: Vec<u32> = (0..nnz as u32 * 100).step_by(100).collect();
        let values: Vec<f32> = vec![1.0; nnz];

        group.bench_with_input(
            BenchmarkId::new("new", nnz),
            &(indices.clone(), values.clone(), dim),
            |b, (i, v, d)| {
                b.iter(|| {
                    SparseVector::new(
                        black_box(i.clone()),
                        black_box(v.clone()),
                        black_box(*d)
                    )
                });
            },
        );
    }

    // Benchmark from_pairs() with unsorted data
    for nnz in [50, 100, 200] {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut indices: Vec<u32> = (0..dim).collect();
        indices.shuffle(&mut rng);
        indices.truncate(nnz);
        // Don't sort - let from_pairs do it

        let pairs: Vec<(u32, f32)> = indices
            .iter()
            .map(|&i| (i, rng.gen_range(-1.0..1.0)))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("from_pairs", nnz),
            &(pairs.clone(), dim),
            |b, (p, d)| {
                b.iter(|| {
                    SparseVector::from_pairs(black_box(p), black_box(*d))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark normalization.
fn bench_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_normalize");
    group.sample_size(500);

    let dim = 10_000u32;

    for nnz in [50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("nnz_{}", nnz)),
            &nnz,
            |b, &nnz| {
                let v = random_sparse(dim, nnz, 42);

                b.iter(|| {
                    v.normalize()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_dot_product,
    bench_dot_product_overlap,
    bench_cosine,
    bench_norm,
    bench_construction,
    bench_normalize
);
criterion_main!(benches);
```

**Cargo.toml Addition:**

```toml
[[bench]]
name = "sparse_bench"
harness = false

[dev-dependencies]
rand = "0.8"
rand_chacha = "0.3"
```

**Acceptance Criteria:**
- [ ] `benches/sparse_bench.rs` created
- [ ] `rand` and `rand_chacha` in dev-dependencies
- [ ] Benchmark groups for dot product, cosine, norm, construction
- [ ] Sample size >= 1000 for accurate percentiles
- [ ] Deterministic RNG for reproducibility

**Estimated Duration:** 45 minutes

**Agent:** BENCHMARK_SCIENTIST

---

### W37.6.2: Run Benchmarks

**Objective:** Execute benchmarks and document results.

**Commands:**

```bash
# Run all sparse benchmarks
cargo bench --bench sparse_bench --features sparse

# Run specific benchmark group
cargo bench --bench sparse_bench --features sparse -- sparse_dot_product

# Generate HTML report
cargo bench --bench sparse_bench --features sparse -- --save-baseline week37
```

**Expected Output Format:**

```
sparse_dot_product/nnz_50
                        time:   [245.23 ns 248.67 ns 252.89 ns]
sparse_dot_product/nnz_100
                        time:   [512.45 ns 521.34 ns 531.02 ns]
```

**Results Documentation:**

Create `docs/benchmarks/2026-01-17_sparse_metrics.md`:

```markdown
# Sparse Metrics Benchmark Results

**Date:** 2026-01-17
**Hardware:** [CPU Model], [RAM], [OS]
**Commit:** [git hash]

## Dot Product Performance

| NNZ | P50 | P99 | Target P50 | Target P99 | Status |
|:----|:----|:----|:-----------|:-----------|:-------|
| 50  | XXX ns | XXX ns | <300ns | <500ns | ✅/❌ |
| 100 | XXX ns | XXX ns | <600ns | <1μs | ✅/❌ |

## Cosine Similarity Performance

| NNZ | P50 | P99 |
|:----|:----|:----|
| 50  | XXX ns | XXX ns |
| 100 | XXX ns | XXX ns |

## Analysis

[Performance observations and recommendations]

## Conclusion

[Whether RFC-007 targets are met]
```

**Acceptance Criteria:**
- [ ] Benchmarks run successfully
- [ ] Results documented in `docs/benchmarks/`
- [ ] Hardware/environment documented
- [ ] Comparison to RFC-007 targets

**Estimated Duration:** 30 minutes

**Agent:** BENCHMARK_SCIENTIST

---

### W37.6.3: Submit for Hostile Review

**Objective:** Get HOSTILE_REVIEWER approval for sparse module.

**Review Scope:**

```
src/sparse/
├── mod.rs
├── error.rs
├── vector.rs
└── metrics.rs

tests/
├── sparse_vector_test.rs
└── sparse_metrics_test.rs

benches/
└── sparse_bench.rs
```

**Pre-Review Checklist:**

```bash
# Ensure all tests pass
cargo test --features sparse

# Ensure clippy clean
cargo clippy --features sparse -- -D warnings

# Ensure docs build
cargo doc --features sparse --no-deps

# Ensure benchmarks run
cargo bench --bench sparse_bench --features sparse -- --test
```

**Invoke Review:**

```
/review src/sparse/
```

**Expected Review Criteria:**

1. **Correctness Attack:**
   - All unit tests pass
   - Property tests verify invariants
   - Cross-check with dense computation

2. **Safety Attack:**
   - No `unsafe` code
   - No panics (all errors handled via Result)
   - No unwrap in library code

3. **Performance Attack:**
   - Benchmarks meet RFC-007 targets
   - Complexity is documented
   - No unnecessary allocations

4. **Maintainability Attack:**
   - All public items documented
   - Examples in doc comments
   - Consistent naming

**Acceptance Criteria:**
- [ ] Pre-review checklist complete
- [ ] `/review` command invoked
- [ ] All critical issues addressed
- [ ] HOSTILE_REVIEWER verdict: APPROVED

**Estimated Duration:** 45 minutes

**Agent:** HOSTILE_REVIEWER

---

## Day 6 Checklist

- [ ] W37.6.1: `benches/sparse_bench.rs` created
- [ ] W37.6.2: Benchmarks run, results documented
- [ ] W37.6.3: Hostile review APPROVED
- [ ] Dot product (50 nnz) P99 < 500ns
- [ ] Dot product (100 nnz) P99 < 1μs
- [ ] All tests pass
- [ ] Clippy clean

## Day 6 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Performance targets met | Benchmark results |
| HOSTILE_REVIEWER APPROVED | Review document |
| All tests pass | `cargo test --features sparse` |
| Clippy clean | `cargo clippy -- -D warnings` |

## Day 6 Verification Commands

```bash
# Full verification sequence
cargo fmt --check
cargo clippy --features sparse -- -D warnings
cargo test --features sparse
cargo bench --bench sparse_bench --features sparse
cargo doc --features sparse --no-deps
```

## Day 6 Handoff

After completing Day 6:

**Artifacts Generated:**
- `benches/sparse_bench.rs`
- `docs/benchmarks/2026-01-17_sparse_metrics.md`
- `docs/reviews/2026-01-17_sparse_module_APPROVED.md` (from hostile review)

**Status:** COMPLETE (if approved)

**Next:** Week 38 — SparseStorage Implementation

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

*Agent: PLANNER + BENCHMARK_SCIENTIST + HOSTILE_REVIEWER*
*Status: [PROPOSED]*
*Date: 2026-01-08*

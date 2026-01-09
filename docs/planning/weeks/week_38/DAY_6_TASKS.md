# Week 38 Day 6: Benchmarks + Hostile Review

**Date:** 2026-01-24
**Focus:** Validate SparseStorage performance targets and get hostile review
**Estimated Duration:** 2 hours
**Phase:** RFC-007 Implementation Phase 2 (Storage)
**Dependencies:** Days 1-5 — ALL MUST BE COMPLETE

---

## Tasks

### W38.6.1: Create Benchmark File

**Objective:** Create `benches/sparse_storage_bench.rs` with criterion benchmarks.

**Rust Implementation:**

```rust
// benches/sparse_storage_bench.rs

//! Benchmarks for sparse storage operations.
//!
//! Performance targets from RFC-007:
//! - Insert: P50 <50us, P99 <100us
//! - Get: <1us
//! - Iteration: <100ms for 100k vectors

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::sparse::{SparseStorage, SparseVector};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::time::Duration;

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
    let values: Vec<f32> = (0..nnz).map(|_| rng.gen_range(-1.0..1.0)).collect();

    SparseVector::new(indices, values, dim).expect("Generated vector should be valid")
}

/// Create storage pre-populated with N sparse vectors.
fn create_storage(count: usize, dim: u32, nnz: usize) -> SparseStorage {
    let mut storage = SparseStorage::new();
    for i in 0..count {
        let vec = random_sparse(dim, nnz, i as u64);
        storage.insert(&vec).expect("Insert should succeed");
    }
    storage
}

// =============================================================================
// BENCHMARK GROUP: storage_insert
// =============================================================================

/// Benchmark insert at various batch sizes.
fn bench_storage_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_insert");

    // Configure for accurate P99 measurement
    group.sample_size(500);
    group.measurement_time(Duration::from_secs(10));

    let dim = 10_000u32;
    let nnz = 50usize;

    // Insert single vector
    group.throughput(Throughput::Elements(1));
    group.bench_function("single", |b| {
        let mut storage = SparseStorage::new();
        let vec = random_sparse(dim, nnz, 42);

        b.iter(|| {
            storage.insert(black_box(&vec)).unwrap()
        });
    });

    // Insert into storage with existing vectors (simulates steady state)
    for initial_count in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("into_existing", initial_count),
            &initial_count,
            |b, &count| {
                let mut storage = create_storage(count, dim, nnz);
                let vec = random_sparse(dim, nnz, 999);

                b.iter(|| {
                    storage.insert(black_box(&vec)).unwrap()
                });
            },
        );
    }

    // Batch insert throughput
    for batch_size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            &batch_size,
            |b, &count| {
                let vectors: Vec<SparseVector> = (0..count)
                    .map(|i| random_sparse(dim, nnz, i as u64))
                    .collect();

                b.iter(|| {
                    let mut storage = SparseStorage::new();
                    for vec in &vectors {
                        storage.insert(black_box(vec)).unwrap();
                    }
                    storage
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_get
// =============================================================================

/// Benchmark get operations.
fn bench_storage_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_get");

    group.sample_size(1000);
    group.measurement_time(Duration::from_secs(5));

    let dim = 10_000u32;
    let nnz = 50usize;

    // Get from 10k storage (matches RFC-007 target scenario)
    let storage = create_storage(10_000, dim, nnz);

    // Get random vector
    group.bench_function("random_from_10k", |b| {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        b.iter(|| {
            let id = rng.gen_range(0..10_000u64);
            storage.get(black_box(id))
        });
    });

    // Get first vector (cache-friendly)
    group.bench_function("first_from_10k", |b| {
        b.iter(|| storage.get(black_box(0)));
    });

    // Get last vector (potential cache-miss)
    group.bench_function("last_from_10k", |b| {
        b.iter(|| storage.get(black_box(9999)));
    });

    // Get non-existent (should return None quickly)
    group.bench_function("missing_from_10k", |b| {
        b.iter(|| storage.get(black_box(99999)));
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_iter
// =============================================================================

/// Benchmark iteration over storage.
fn bench_storage_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_iter");

    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));

    let dim = 10_000u32;
    let nnz = 50usize;

    // Iterate over various storage sizes
    for count in [1000, 10000, 100000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("count_{count}")),
            &count,
            |b, &count| {
                let storage = create_storage(count, dim, nnz);

                b.iter(|| {
                    let mut sum = 0.0f32;
                    for (id, vec) in storage.iter() {
                        // Access both id and vector to ensure full iteration
                        sum += black_box(id) as f32 + vec.nnz() as f32;
                    }
                    sum
                });
            },
        );
    }

    // Iterate with filter (only non-deleted)
    let mut storage_with_deletions = create_storage(10_000, dim, nnz);
    // Delete every 10th vector
    for i in (0..10_000).step_by(10) {
        let _ = storage_with_deletions.delete(i);
    }

    group.bench_function("10k_with_deletions", |b| {
        b.iter(|| {
            let mut sum = 0.0f32;
            for (id, vec) in storage_with_deletions.iter() {
                sum += black_box(id) as f32 + vec.nnz() as f32;
            }
            sum
        });
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_roundtrip
// =============================================================================

/// Benchmark serialization roundtrip.
fn bench_storage_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_roundtrip");

    group.sample_size(50);
    group.measurement_time(Duration::from_secs(15));

    let dim = 10_000u32;
    let nnz = 50usize;

    // Roundtrip at various sizes
    for count in [1000, 10000] {
        group.throughput(Throughput::Elements(count as u64));

        // Serialize benchmark
        group.bench_with_input(
            BenchmarkId::new("serialize", count),
            &count,
            |b, &count| {
                let storage = create_storage(count, dim, nnz);

                b.iter(|| {
                    storage.to_bytes()
                });
            },
        );

        // Deserialize benchmark
        let storage = create_storage(count, dim, nnz);
        let bytes = storage.to_bytes();

        group.bench_with_input(
            BenchmarkId::new("deserialize", count),
            &bytes,
            |b, bytes| {
                b.iter(|| {
                    SparseStorage::from_bytes(black_box(bytes))
                });
            },
        );

        // Full roundtrip
        group.bench_with_input(
            BenchmarkId::new("full_roundtrip", count),
            &count,
            |b, &count| {
                let storage = create_storage(count, dim, nnz);

                b.iter(|| {
                    let bytes = storage.to_bytes();
                    SparseStorage::from_bytes(black_box(&bytes))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_delete
// =============================================================================

/// Benchmark delete operations.
fn bench_storage_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_delete");

    group.sample_size(500);

    let dim = 10_000u32;
    let nnz = 50usize;

    // Single delete
    group.bench_function("single", |b| {
        b.iter_batched(
            || create_storage(1000, dim, nnz),
            |mut storage| {
                storage.delete(black_box(500)).unwrap()
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Batch delete (delete 10% of vectors)
    group.bench_function("batch_10_percent", |b| {
        b.iter_batched(
            || create_storage(10_000, dim, nnz),
            |mut storage| {
                for i in (0..10_000).step_by(10) {
                    let _ = storage.delete(i);
                }
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_memory
// =============================================================================

/// Benchmark memory allocation patterns.
fn bench_storage_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_memory");

    group.sample_size(100);

    let dim = 10_000u32;

    // Memory with varying sparsity
    for nnz in [10, 50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::new("create_10k", nnz),
            &nnz,
            |b, &nnz| {
                b.iter(|| {
                    create_storage(10_000, dim, nnz)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_storage_insert,
    bench_storage_get,
    bench_storage_iter,
    bench_storage_roundtrip,
    bench_storage_delete,
    bench_storage_memory
);
criterion_main!(benches);
```

**Cargo.toml Addition:**

```toml
[[bench]]
name = "sparse_storage_bench"
harness = false
```

**Acceptance Criteria:**
- [ ] `benches/sparse_storage_bench.rs` created
- [ ] Benchmark groups: `storage_insert`, `storage_get`, `storage_iter`, `storage_roundtrip`
- [ ] Sample size >= 500 for accurate percentiles
- [ ] Deterministic RNG for reproducibility
- [ ] Throughput annotations for batch operations

**Estimated Duration:** 45 minutes

**Agent:** BENCHMARK_SCIENTIST

---

### W38.6.2: Run Benchmarks and Document Results

**Objective:** Execute benchmarks and document results.

**Commands:**

```bash
# Run all sparse storage benchmarks
cargo bench --bench sparse_storage_bench --features sparse

# Run specific benchmark group
cargo bench --bench sparse_storage_bench --features sparse -- storage_insert
cargo bench --bench sparse_storage_bench --features sparse -- storage_get
cargo bench --bench sparse_storage_bench --features sparse -- storage_iter
cargo bench --bench sparse_storage_bench --features sparse -- storage_roundtrip

# Generate HTML report with baseline
cargo bench --bench sparse_storage_bench --features sparse -- --save-baseline week38
```

**Expected Output Format:**

```
storage_insert/single
                        time:   [42.123 us 43.456 us 44.789 us]
storage_get/random_from_10k
                        time:   [0.812 us 0.834 us 0.867 us]
storage_iter/count_100000
                        time:   [78.234 ms 82.456 ms 86.789 ms]
```

**Results Documentation:**

Create `docs/benchmarks/2026-01-24_sparse_storage.md`:

```markdown
# Sparse Storage Benchmark Results

**Date:** 2026-01-24
**Hardware:** [CPU Model], [RAM], [OS]
**Commit:** [git hash]
**Phase:** RFC-007 Phase 2 (SparseStorage)

## Insert Performance

| Scenario | P50 | P99 | Target P50 | Target P99 | Status |
|:---------|:----|:----|:-----------|:-----------|:-------|
| Single insert | XXX us | XXX us | <50us | <100us | ?/? |
| Into 100 existing | XXX us | XXX us | <50us | <100us | ?/? |
| Into 1000 existing | XXX us | XXX us | <50us | <100us | ?/? |
| Into 10000 existing | XXX us | XXX us | <50us | <100us | ?/? |

## Get Performance

| Scenario | P50 | P99 | Target | Status |
|:---------|:----|:----|:-------|:-------|
| Random from 10k | XXX ns | XXX ns | <1us | ?/? |
| First from 10k | XXX ns | XXX ns | <1us | ?/? |
| Last from 10k | XXX ns | XXX ns | <1us | ?/? |
| Missing key | XXX ns | XXX ns | <1us | ?/? |

## Iteration Performance

| Count | Time | Target | Status |
|:------|:-----|:-------|:-------|
| 1,000 | XXX ms | N/A | N/A |
| 10,000 | XXX ms | N/A | N/A |
| 100,000 | XXX ms | <100ms | ?/? |

## Serialization Roundtrip

| Count | Serialize | Deserialize | Total |
|:------|:----------|:------------|:------|
| 1,000 | XXX ms | XXX ms | XXX ms |
| 10,000 | XXX ms | XXX ms | XXX ms |

## Analysis

### Insert Performance
[Observations about insert latency and scaling]

### Get Performance
[Observations about get latency]

### Iteration Performance
[Observations about iteration overhead]

### Memory Overhead
[Estimated memory per vector based on benchmarks]

## RFC-007 Target Compliance

| Target | Requirement | Measured | Status |
|:-------|:------------|:---------|:-------|
| Insert P50 | <50us | XXX us | ?/? |
| Insert P99 | <100us | XXX us | ?/? |
| Get | <1us | XXX ns | ?/? |
| Iterate 100k | <100ms | XXX ms | ?/? |

## Conclusion

[Whether RFC-007 storage targets are met]

## Recommendations

[Any optimization suggestions for Phase 3 or later]
```

**Acceptance Criteria:**
- [ ] Benchmarks run successfully
- [ ] Results documented in `docs/benchmarks/2026-01-24_sparse_storage.md`
- [ ] Hardware/environment documented
- [ ] Comparison to RFC-007 targets
- [ ] All benchmark groups executed

**Estimated Duration:** 30 minutes

**Agent:** BENCHMARK_SCIENTIST

---

### W38.6.3: Submit for Hostile Review

**Objective:** Get HOSTILE_REVIEWER approval for SparseStorage implementation.

**Review Scope:**

```
src/sparse/
|-- mod.rs          (updated exports)
|-- error.rs        (updated with storage errors)
|-- vector.rs       (unchanged from Week 37)
|-- metrics.rs      (unchanged from Week 37)
|-- storage.rs      (NEW - Week 38)

tests/
|-- sparse_vector_test.rs    (unchanged)
|-- sparse_metrics_test.rs   (unchanged)
|-- sparse_storage_test.rs   (NEW - Week 38)

benches/
|-- sparse_bench.rs          (unchanged)
|-- sparse_storage_bench.rs  (NEW - Week 38)
```

**Pre-Review Checklist:**

```bash
# Ensure all tests pass
cargo test --features sparse

# Ensure clippy clean
cargo clippy --features sparse -- -D warnings

# Ensure docs build
cargo doc --features sparse --no-deps

# Ensure benchmarks compile and run
cargo bench --bench sparse_storage_bench --features sparse -- --test

# Check storage-specific tests
cargo test --features sparse sparse_storage

# Verify serialization roundtrip
cargo test --features sparse test_storage_roundtrip
```

**Invoke Review:**

```
/review src/sparse/storage.rs
```

**Expected Review Criteria:**

1. **Correctness Attack:**
   - Insert returns unique, monotonic IDs
   - Get returns correct vector for given ID
   - Delete marks vector as deleted (not physical removal)
   - Iteration skips deleted vectors
   - Serialization preserves all data

2. **Safety Attack:**
   - No `unsafe` code
   - No panics (all errors handled via Result)
   - No `unwrap()` in library code
   - Bounds checking on all array accesses
   - No data races (single-threaded design)

3. **Performance Attack:**
   - Insert: <50us P50, <100us P99
   - Get: <1us
   - Iteration: <100ms for 100k vectors
   - No unnecessary allocations
   - Efficient packed array layout

4. **API Attack:**
   - Consistent with RFC-007 spec
   - Error types are descriptive
   - Public API is minimal and focused
   - Methods are well-documented

5. **Maintainability Attack:**
   - All public items documented
   - Examples in doc comments
   - Consistent naming with existing codebase
   - Clear separation of concerns

**Acceptance Criteria:**
- [ ] Pre-review checklist complete
- [ ] `/review` command invoked
- [ ] All critical issues addressed (if any)
- [ ] All major issues addressed (if any)
- [ ] HOSTILE_REVIEWER verdict: APPROVED

**Estimated Duration:** 45 minutes

**Agent:** HOSTILE_REVIEWER

---

## Day 6 Checklist

- [ ] W38.6.1: `benches/sparse_storage_bench.rs` created
- [ ] W38.6.2: Benchmarks run, results documented
- [ ] W38.6.3: Hostile review APPROVED
- [ ] Insert P50 < 50us
- [ ] Insert P99 < 100us
- [ ] Get < 1us
- [ ] Iteration 100k < 100ms
- [ ] All tests pass
- [ ] Clippy clean

## Day 6 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Benchmarks created | `benches/sparse_storage_bench.rs` exists |
| Performance targets met | Benchmark results in docs |
| HOSTILE_REVIEWER APPROVED | Review document exists |
| All tests pass | `cargo test --features sparse` |
| Clippy clean | `cargo clippy -- -D warnings` |

## Day 6 Verification Commands

```bash
# Full verification sequence
cargo fmt --check
cargo clippy --features sparse -- -D warnings
cargo test --features sparse
cargo bench --bench sparse_storage_bench --features sparse
cargo doc --features sparse --no-deps
```

## Day 6 Handoff

After completing Day 6:

**Artifacts Generated:**
- `benches/sparse_storage_bench.rs`
- `docs/benchmarks/2026-01-24_sparse_storage.md`
- `docs/reviews/2026-01-24_sparse_storage_APPROVED.md` (from hostile review)

**Status:** COMPLETE (if approved)

**Next:** Week 39 — Sparse Search Implementation (RFC-007 Phase 3)

---

## Week 38 Exit Criteria

Week 38 is complete when:

- [ ] SparseStorage fully implemented with insert/get/delete
- [ ] Serialization roundtrip works correctly
- [ ] Benchmarks meet RFC-007 targets:
  - [ ] Insert: <50us P50, <100us P99
  - [ ] Get: <1us
  - [ ] Iteration: <100ms for 100k vectors
- [ ] Hostile review APPROVED
- [ ] Week 39 plan created

---

## Week 38 Summary

| Day | Focus | Key Deliverable |
|:----|:------|:----------------|
| 1 | Storage Structure | `SparseStorage` struct with packed arrays |
| 2 | Insert + Get | Core CRUD operations |
| 3 | Delete + Iteration | Soft delete with BitVec, efficient iteration |
| 4 | Serialization | `to_bytes()` / `from_bytes()` roundtrip |
| 5 | Integration + Cleanup | Wire into `src/sparse/mod.rs`, full test suite |
| 6 | Benchmarks + Review | Performance validation, hostile review |

---

*Agent: PLANNER + BENCHMARK_SCIENTIST + HOSTILE_REVIEWER*
*Status: [PROPOSED]*
*Date: 2026-01-09*

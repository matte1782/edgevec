# Sparse Module Structure Plan

**Date:** 2026-01-08
**Author:** PLANNER
**Status:** [PROPOSED]
**RFC:** RFC-007 (APPROVED)

---

## Module Layout

```
src/sparse/
├── mod.rs              # Module exports, feature flags
├── vector.rs           # SparseVector struct, validation
├── metrics.rs          # dot_product, cosine, norm
├── storage.rs          # SparseStorage with packed arrays
├── search.rs           # Brute-force sparse search
├── inverted.rs         # Optional inverted index (Phase 3)
├── hybrid.rs           # HybridStorage, fusion algorithms
└── error.rs            # SparseError enum

src/wasm/
└── sparse.rs           # WASM bindings for SparseVector

tests/
├── sparse_vector_test.rs
├── sparse_metrics_test.rs
├── sparse_storage_test.rs
├── sparse_search_test.rs
└── hybrid_search_test.rs

benches/
└── sparse_bench.rs     # Dot product, search benchmarks
```

---

## File Details

### `src/sparse/mod.rs`

```rust
//! Sparse vector support for hybrid search.
//!
//! # Feature Flags
//! - `sparse` (default): Core sparse types and metrics
//! - `sparse-inverted`: Inverted index for fast sparse search

mod vector;
mod metrics;
mod storage;
mod search;
mod error;

#[cfg(feature = "sparse-inverted")]
mod inverted;

pub use vector::SparseVector;
pub use metrics::{sparse_dot_product, sparse_cosine};
pub use storage::{SparseStorage, SparseId};
pub use search::SparseSearcher;
pub use error::SparseError;

#[cfg(feature = "sparse-inverted")]
pub use inverted::SparseInvertedIndex;
```

### `src/sparse/vector.rs`

```rust
//! SparseVector type using CSR format.

use serde::{Deserialize, Serialize};
use crate::sparse::error::SparseError;

/// Sparse vector using Compressed Sparse Row (CSR) format.
///
/// # Memory Layout
/// - indices: [u32; N] — sorted positions of non-zero elements
/// - values: [f32; N] — corresponding values
/// - dim: u32 — maximum dimension (vocabulary/feature space size)
///
/// # Invariants
/// - indices are sorted ascending
/// - no duplicate indices
/// - no NaN or Infinity in values
/// - nnz >= 1 (no empty vectors)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SparseVector {
    indices: Vec<u32>,
    values: Vec<f32>,
    dim: u32,
}

impl SparseVector {
    /// Create from pre-sorted indices and values.
    pub fn new(indices: Vec<u32>, values: Vec<f32>, dim: u32) -> Result<Self, SparseError>;

    /// Create from unsorted pairs. Sorts internally.
    pub fn from_pairs(pairs: &[(u32, f32)], dim: u32) -> Result<Self, SparseError>;

    /// Create with single element.
    pub fn singleton(index: u32, value: f32, dim: u32) -> Result<Self, SparseError>;

    // Accessors
    pub fn indices(&self) -> &[u32];
    pub fn values(&self) -> &[f32];
    pub fn dim(&self) -> u32;
    pub fn nnz(&self) -> usize;

    // Operations
    pub fn dot(&self, other: &SparseVector) -> f32;
    pub fn cosine(&self, other: &SparseVector) -> f32;
    pub fn norm(&self) -> f32;
    pub fn normalize(&self) -> Result<Self, SparseError>;
}
```

### `src/sparse/metrics.rs`

```rust
//! Distance metrics for sparse vectors.

use crate::sparse::SparseVector;
use std::cmp::Ordering;

/// Sparse dot product using merge-intersection.
/// Complexity: O(|a| + |b|) worst case.
#[inline]
pub fn sparse_dot_product(a: &SparseVector, b: &SparseVector) -> f32;

/// Sparse cosine similarity.
/// Returns value in [-1, 1] for normalized vectors, 0 for zero vectors.
#[inline]
pub fn sparse_cosine(a: &SparseVector, b: &SparseVector) -> f32;

/// L2 norm of sparse vector.
#[inline]
pub fn sparse_norm(v: &SparseVector) -> f32;
```

### `src/sparse/storage.rs`

```rust
//! Packed storage for sparse vectors.

use bitvec::prelude::*;
use crate::sparse::{SparseVector, SparseError};

/// Unique identifier for sparse vectors.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SparseId(pub u64);

/// Packed storage for multiple sparse vectors.
///
/// # Memory Layout
/// All vectors' indices and values are concatenated.
/// Offsets array tracks where each vector starts.
pub struct SparseStorage {
    indices: Vec<u32>,
    values: Vec<f32>,
    offsets: Vec<u32>,
    dims: Vec<u32>,
    deleted: BitVec,
    next_id: u64,
}

impl SparseStorage {
    pub fn new() -> Self;
    pub fn insert(&mut self, vector: &SparseVector) -> Result<SparseId, SparseError>;
    pub fn get(&self, id: SparseId) -> Option<SparseVector>;
    pub fn delete(&mut self, id: SparseId) -> Result<bool, SparseError>;
    pub fn is_deleted(&self, id: SparseId) -> bool;
    pub fn len(&self) -> usize;
    pub fn live_count(&self) -> usize;
}
```

### `src/sparse/search.rs`

```rust
//! Sparse vector search.

use crate::sparse::{SparseVector, SparseStorage, SparseId};
use crate::SearchResult;

/// Sparse vector searcher (brute force).
pub struct SparseSearcher<'a> {
    storage: &'a SparseStorage,
}

impl<'a> SparseSearcher<'a> {
    pub fn new(storage: &'a SparseStorage) -> Self;

    /// Brute-force search by dot product.
    /// Returns top-k results sorted by score descending.
    pub fn search(&self, query: &SparseVector, k: usize) -> Vec<SearchResult>;
}
```

### `src/sparse/error.rs`

```rust
//! Error types for sparse operations.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SparseError {
    #[error("indices must be sorted ascending")]
    UnsortedIndices,

    #[error("duplicate index at position {0}")]
    DuplicateIndex(usize),

    #[error("index {index} exceeds dimension {dim}")]
    IndexOutOfBounds { index: u32, dim: u32 },

    #[error("value at index {0} is NaN or Infinity")]
    InvalidValue(usize),

    #[error("sparse vector must have at least one element")]
    EmptyVector,

    #[error("indices and values length mismatch: {indices} vs {values}")]
    LengthMismatch { indices: usize, values: usize },

    #[error("sparse ID {0} not found")]
    IdNotFound(u64),

    #[error("cannot normalize zero vector")]
    ZeroNorm,
}
```

---

## Test Files

### `tests/sparse_vector_test.rs`

```rust
//! Unit tests for SparseVector.

#[test]
fn test_new_valid() { /* sorted indices, valid values */ }

#[test]
fn test_new_unsorted_fails() { /* error on unsorted */ }

#[test]
fn test_new_duplicate_fails() { /* error on duplicate index */ }

#[test]
fn test_new_nan_fails() { /* error on NaN value */ }

#[test]
fn test_new_empty_fails() { /* error on zero nnz */ }

#[test]
fn test_from_pairs_sorts() { /* auto-sorting */ }

#[test]
fn test_singleton() { /* single element */ }
```

### `tests/sparse_metrics_test.rs`

```rust
//! Property tests for sparse metrics.

use proptest::prelude::*;

proptest! {
    #[test]
    fn dot_commutative(a: SparseVector, b: SparseVector) {
        prop_assert_eq!(a.dot(&b), b.dot(&a));
    }

    #[test]
    fn dot_positive_semidefinite(v: SparseVector) {
        prop_assert!(v.dot(&v) >= 0.0);
    }

    #[test]
    fn cosine_self_is_one(v: NonZeroSparseVector) {
        prop_assert!((v.cosine(&v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_in_range(a: SparseVector, b: SparseVector) {
        let c = a.cosine(&b);
        prop_assert!(c >= -1.0 && c <= 1.0);
    }
}
```

---

## Benchmark File

### `benches/sparse_bench.rs`

```rust
//! Sparse vector benchmarks.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_dot");

    for nnz in [10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(nnz),
            &nnz,
            |b, &nnz| {
                let a = random_sparse(10000, nnz);
                let query = random_sparse(10000, nnz);
                b.iter(|| a.dot(&query));
            }
        );
    }
    group.finish();
}

fn bench_search_brute(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_search");

    for n in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &n,
            |b, &n| {
                let storage = create_storage(n, 50);
                let query = random_sparse(10000, 50);
                let searcher = SparseSearcher::new(&storage);
                b.iter(|| searcher.search(&query, 10));
            }
        );
    }
    group.finish();
}

criterion_group!(benches, bench_dot_product, bench_search_brute);
criterion_main!(benches);
```

---

## Implementation Order

| Week | Files | Tests | Benchmarks |
|:-----|:------|:------|:-----------|
| 37 | `error.rs`, `vector.rs`, `metrics.rs` | `sparse_vector_test.rs`, `sparse_metrics_test.rs` | `sparse_bench.rs` (dot only) |
| 38 | `storage.rs` | `sparse_storage_test.rs` | - |
| 39 | `search.rs` | `sparse_search_test.rs` | `sparse_bench.rs` (search) |
| 40 | `hybrid.rs` | `hybrid_search_test.rs` | - |
| 41 | `src/wasm/sparse.rs` | WASM integration tests | - |

---

## Dependencies

### New Crates (None Required)

All functionality uses existing dependencies:
- `serde` — Serialization (already in Cargo.toml)
- `bitvec` — Deletion bitmap (already in Cargo.toml)
- `thiserror` — Error handling (already in Cargo.toml)

### Feature Flags

```toml
[features]
default = ["sparse"]
sparse = []
sparse-inverted = ["sparse"]
```

---

## Success Criteria

- [ ] `SparseVector::new` validates all invariants
- [ ] `sparse_dot_product` passes property tests
- [ ] Dot product (50 nnz) P99 < 500ns
- [ ] Search (100k, k=10) P99 < 5ms
- [ ] All tests pass: `cargo test --features sparse`
- [ ] Benchmarks exist in `benches/sparse_bench.rs`

---

**Status:** [PROPOSED] — Ready for Week 37 implementation

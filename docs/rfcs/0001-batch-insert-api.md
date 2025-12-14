# RFC 0001: Batch Insert API

**Status:** PROPOSED
**Author:** RUST_ENGINEER
**Date:** 2025-12-13
**Task:** W10.5

---

## Summary

This RFC defines the public API for batch insert operations in EdgeVec. The goal is to enable efficient bulk loading of vectors while maintaining WASM compatibility and staying within a 100MB memory budget for 10,000 vectors.

---

## Motivation

Currently, EdgeVec only supports single-vector insertion via `HnswIndex::insert()`. For real-world use cases like:
- Initial index population from a dataset
- Bulk updates after model retraining
- Migration from other vector databases

A batch insert API is essential for performance. Inserting 10,000 vectors one-by-one would require 10,000 separate calls with associated overhead.

---

## Design Goals

| Goal | Rationale |
|:-----|:----------|
| **WASM Compatible** | No `std::thread`, must work in browser |
| **Memory Bounded** | <100MB for 10k vectors (768 dimensions) |
| **Progress Reporting** | UI feedback for long operations |
| **Error Strategy** | Clear semantics for partial failures |
| **API Consistency** | Matches existing `insert()` pattern |

---

## Proposed API

### 1. Core Trait: `BatchInsertable`

```rust
use crate::hnsw::{GraphError, VectorId};
use crate::storage::VectorStorage;

/// Error type for batch insert operations.
#[derive(Debug, Clone)]
pub struct BatchError {
    /// Number of vectors successfully inserted before failure.
    pub successful_count: usize,

    /// IDs of successfully inserted vectors.
    pub successful_ids: Vec<VectorId>,

    /// The underlying error that caused the failure.
    pub cause: GraphError,

    /// Index of the vector that caused the failure (if applicable).
    pub failed_index: Option<usize>,
}

impl std::fmt::Display for BatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "batch insert failed after {} vectors: {}",
            self.successful_count, self.cause
        )
    }
}

impl std::error::Error for BatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

/// Result type for batch insert operations.
pub type BatchResult = Result<Vec<VectorId>, BatchError>;

/// Trait for batch insert operations.
pub trait BatchInsertable {
    /// Inserts multiple vectors in a single operation.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Slice of vector slices to insert.
    /// * `storage` - Mutable reference to vector storage.
    ///
    /// # Returns
    ///
    /// On success: `Ok(Vec<VectorId>)` with IDs in insertion order.
    /// On failure: `Err(BatchError)` with partial success information.
    ///
    /// # Error Strategy
    ///
    /// Uses **fail-fast**: stops at first error and returns partial results.
    /// This ensures atomicity of the error state - you know exactly which
    /// vectors succeeded.
    fn insert_batch(
        &mut self,
        vectors: &[&[f32]],
        storage: &mut VectorStorage,
    ) -> BatchResult;

    /// Inserts multiple vectors with progress reporting.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Slice of vector slices to insert.
    /// * `storage` - Mutable reference to vector storage.
    /// * `progress` - Callback invoked after each vector: `(completed, total)`.
    ///
    /// # Progress Callback
    ///
    /// The callback is invoked AFTER each successful insert. If insertion
    /// fails, the callback is NOT invoked for the failed vector.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// index.insert_batch_with_progress(&vectors, &mut storage, |done, total| {
    ///     println!("Progress: {}/{} ({}%)", done, total, done * 100 / total);
    /// })?;
    /// ```
    fn insert_batch_with_progress<F>(
        &mut self,
        vectors: &[&[f32]],
        storage: &mut VectorStorage,
        progress: F,
    ) -> BatchResult
    where
        F: FnMut(usize, usize);
}
```

### 2. Implementation for `HnswIndex`

```rust
impl BatchInsertable for HnswIndex {
    fn insert_batch(
        &mut self,
        vectors: &[&[f32]],
        storage: &mut VectorStorage,
    ) -> BatchResult {
        self.insert_batch_with_progress(vectors, storage, |_, _| {})
    }

    fn insert_batch_with_progress<F>(
        &mut self,
        vectors: &[&[f32]],
        storage: &mut VectorStorage,
        mut progress: F,
    ) -> BatchResult
    where
        F: FnMut(usize, usize),
    {
        let total = vectors.len();
        let mut ids = Vec::with_capacity(total);

        for (i, vector) in vectors.iter().enumerate() {
            match self.insert(vector, storage) {
                Ok(id) => {
                    ids.push(id);
                    progress(i + 1, total);
                }
                Err(cause) => {
                    return Err(BatchError {
                        successful_count: ids.len(),
                        successful_ids: ids,
                        cause,
                        failed_index: Some(i),
                    });
                }
            }
        }

        Ok(ids)
    }
}
```

---

## Error Handling Strategy

### Decision: **Fail-Fast**

| Strategy | Pros | Cons |
|:---------|:-----|:-----|
| **Fail-Fast** | Clear error state, partial results known, simpler implementation | Wastes work on early failure |
| **Partial Success** | Maximizes successful inserts | Complex error reporting, unclear index state |
| **Transactional** | All-or-nothing semantics | Expensive rollback, high memory |

**Rationale for Fail-Fast:**

1. **Clear State**: After failure, you know exactly which vectors succeeded
2. **Recovery Path**: Caller can retry with remaining vectors
3. **Memory Efficiency**: No rollback buffers needed
4. **WASM Simplicity**: No complex transaction management
5. **Caller Control**: Caller can implement partial success wrapper if needed

### Recovery Pattern

```rust
fn insert_with_retry(
    index: &mut HnswIndex,
    vectors: &[&[f32]],
    storage: &mut VectorStorage,
) -> Result<Vec<VectorId>, GraphError> {
    let mut remaining = vectors;
    let mut all_ids = Vec::new();

    loop {
        match index.insert_batch(remaining, storage) {
            Ok(ids) => {
                all_ids.extend(ids);
                break;
            }
            Err(e) => {
                all_ids.extend(e.successful_ids);
                if let Some(failed_idx) = e.failed_index {
                    // Skip failed vector and continue
                    remaining = &remaining[failed_idx + 1..];
                    if remaining.is_empty() {
                        break;
                    }
                    // Log or handle the failure
                    eprintln!("Skipped vector at index {}: {}", failed_idx, e.cause);
                } else {
                    return Err(e.cause);
                }
            }
        }
    }

    Ok(all_ids)
}
```

---

## WASM Memory Budget Analysis

### Assumptions

| Parameter | Value | Source |
|:----------|:------|:-------|
| Batch size | 10,000 vectors | Requirement |
| Dimensions | 768 (typical embedding) | Common use case |
| Storage type | Float32 | Conservative estimate |
| HNSW M | 16 | Default |
| HNSW M0 | 32 | Default (2 * M) |

### Memory Breakdown

#### 1. Input Vectors (Caller's Responsibility)

```
Input: vectors: &[&[f32]]
- Pointer array: 10,000 * 16 bytes = 160 KB
- Vector data (caller owns): 10,000 * 768 * 4 = 30.72 MB

Caller total: ~31 MB
```

#### 2. VectorStorage Growth

```
For each vector:
- data_f32: 768 * 4 bytes = 3,072 bytes
- tombstone: 1 bit = 0.125 bytes

10,000 vectors:
- data_f32: 10,000 * 3,072 = 30.72 MB
- tombstones: 10,000 / 8 = 1.25 KB

Storage total: ~31 MB
```

#### 3. HnswIndex Growth

```
Per node (from DATA_LAYOUT.md):
- HnswNode: 16 bytes
- Neighbor pool (compressed): ~50-100 bytes average

10,000 nodes:
- Nodes: 10,000 * 16 = 160 KB
- Neighbors: 10,000 * 75 = 750 KB (average)

Index total: ~1 MB
```

#### 4. Temporary Allocations During Insert

```
SearchContext (per insert):
- scratch: ~256 candidates * 8 bytes = 2 KB
- neighbor_scratch: ~256 * 4 bytes = 1 KB
- visited: ~1000 entries * 4 bytes = 4 KB

Transient per insert: ~7 KB (reused)
```

### Total Memory Budget

| Component | Size | Notes |
|:----------|:-----|:------|
| Input vectors (caller) | 31 MB | Not counted against library |
| VectorStorage | 31 MB | Persistent growth |
| HnswIndex | 1 MB | Persistent growth |
| Transient | 7 KB | Reused, not cumulative |
| **Library Total** | **~32 MB** | Well under 100 MB |

### Quantized Storage (SQ8)

If using quantized storage:

```
VectorStorage (quantized):
- quantized_data: 10,000 * 768 * 1 = 7.68 MB
- data_f32: 0 (not stored)

Quantized total: ~9 MB
```

**Verdict:** Budget of <100 MB is achievable with significant margin.

---

## Progress Reporting Mechanism

### Design Considerations

| Consideration | Decision |
|:--------------|:---------|
| Callback granularity | Per vector (not per batch of 100) |
| Callback signature | `FnMut(usize, usize)` - (completed, total) |
| Error in callback | Not propagated (callback is for reporting only) |
| WASM compatibility | Closures work in WASM |

### Usage Patterns

#### 1. Console Progress

```rust
index.insert_batch_with_progress(&vectors, &mut storage, |done, total| {
    if done % 100 == 0 || done == total {
        println!("\rInserted {}/{} vectors", done, total);
    }
});
```

#### 2. UI Progress Bar (WASM)

```rust
let progress_callback = Closure::wrap(Box::new(move |done: usize, total: usize| {
    let pct = (done * 100) / total;
    update_progress_bar(pct);
}) as Box<dyn FnMut(usize, usize)>);

index.insert_batch_with_progress(&vectors, &mut storage,
    |done, total| progress_callback.as_ref().unchecked_ref()(done, total)
);
```

#### 3. Cancellation (Future Extension)

```rust
let cancelled = AtomicBool::new(false);

// In WASM, could use a shared flag checked in callback
// For now, callback cannot abort - this is a future consideration
```

---

## Example Usage

### Basic Batch Insert

```rust
use edgevec::hnsw::{HnswConfig, HnswIndex, BatchInsertable};
use edgevec::storage::VectorStorage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let config = HnswConfig::new(768);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage)?;

    // Prepare vectors
    let vectors: Vec<Vec<f32>> = (0..1000)
        .map(|i| (0..768).map(|j| (i * j) as f32 / 1000.0).collect())
        .collect();
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();

    // Batch insert
    let ids = index.insert_batch(&refs, &mut storage)?;

    println!("Inserted {} vectors", ids.len());
    Ok(())
}
```

### With Progress Reporting

```rust
use edgevec::hnsw::{HnswConfig, HnswIndex, BatchInsertable};
use edgevec::storage::VectorStorage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = HnswConfig::new(768);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage)?;

    let vectors: Vec<Vec<f32>> = (0..10_000)
        .map(|i| (0..768).map(|j| (i * j) as f32 / 10000.0).collect())
        .collect();
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();

    let ids = index.insert_batch_with_progress(&refs, &mut storage, |done, total| {
        if done % 1000 == 0 {
            println!("Progress: {}/{} ({:.1}%)", done, total, done as f64 / total as f64 * 100.0);
        }
    })?;

    println!("Inserted {} vectors", ids.len());
    Ok(())
}
```

### Error Handling

```rust
use edgevec::hnsw::{HnswConfig, HnswIndex, BatchInsertable, BatchError};
use edgevec::storage::VectorStorage;

fn handle_batch_insert() {
    let config = HnswConfig::new(4);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Mix of valid and invalid vectors
    let good_vec = vec![1.0, 2.0, 3.0, 4.0];
    let bad_vec = vec![1.0, 2.0]; // Wrong dimensions

    let vectors: Vec<&[f32]> = vec![
        &good_vec,
        &bad_vec, // This will fail
    ];

    match index.insert_batch(&vectors, &mut storage) {
        Ok(ids) => println!("All {} vectors inserted", ids.len()),
        Err(e) => {
            println!("Partial failure after {} vectors", e.successful_count);
            println!("Successful IDs: {:?}", e.successful_ids);
            if let Some(idx) = e.failed_index {
                println!("Failed at index {}: {}", idx, e.cause);
            }
        }
    }
}
```

---

## WASM Considerations

### Thread Safety

- No `std::thread` usage
- Single-threaded batch processing is acceptable for WASM
- Progress callback must not block (WASM is single-threaded)

### Memory Pressure

- WASM memory is capped (typically 4GB max, often 256MB-1GB in practice)
- 32 MB for 10k vectors is well within typical limits
- Consider streaming for very large batches (future extension)

### Future: Web Workers

For W11 implementation, consider optional Web Worker support:
- Main thread handles progress callbacks
- Worker thread handles batch insertion
- Communication via `postMessage`

This is OUT OF SCOPE for W10.5 (design only).

---

## Alternatives Considered

### 1. Iterator-Based API

```rust
fn insert_iter<'a, I>(&mut self, vectors: I) -> BatchResult
where
    I: Iterator<Item = &'a [f32]>;
```

**Rejected:** Less predictable memory usage, progress reporting harder.

### 2. Chunked API

```rust
fn insert_chunked(&mut self, vectors: &[&[f32]], chunk_size: usize) -> BatchResult;
```

**Rejected:** Adds complexity, caller can chunk externally if needed.

### 3. Transactional API

```rust
fn insert_batch_transactional(&mut self, vectors: &[&[f32]]) -> Result<..., ...>;
// Rollback on failure
```

**Rejected:** High memory overhead for rollback state, complex implementation.

---

## Implementation Checklist (for W11.1)

- [ ] Add `BatchError` type to `src/hnsw/graph.rs`
- [ ] Add `BatchInsertable` trait to `src/hnsw/mod.rs`
- [ ] Implement trait for `HnswIndex`
- [ ] Add unit tests for:
  - [ ] Basic batch insert
  - [ ] Empty batch
  - [ ] Single vector batch
  - [ ] Error handling
  - [ ] Progress callback
- [ ] Add integration test with 10k vectors
- [ ] Benchmark vs. sequential insert

---

## Open Questions

1. **Cancellation**: Should progress callback be able to abort the batch?
   - Current decision: No (simplicity)
   - Could add `insert_batch_cancelable()` later

2. **Streaming Large Batches**: What about 1M vectors?
   - Current decision: Out of scope
   - Could add streaming API in future

3. **Parallel Insert**: Should we support concurrent insertion?
   - Current decision: No (WASM compatibility)
   - Could add `rayon` feature flag for native targets

---

## References

- `src/hnsw/insert.rs` - Current single insert implementation
- `src/hnsw/graph.rs` - HnswIndex and GraphError types
- `src/storage.rs` - VectorStorage implementation
- `docs/architecture/DATA_LAYOUT.md` - Memory layout specification

---

## Revision History

| Version | Date | Author | Changes |
|:--------|:-----|:-------|:--------|
| 1.0 | 2025-12-13 | RUST_ENGINEER | Initial RFC |
| 1.1 | 2025-12-13 | RUST_ENGINEER | Implementation revisions (see below) |

---

## Implementation Deviation Record (v1.1)

During Week 11 Day 1 implementation, the following API changes were made from the original RFC design. These changes are documented per HOSTILE_REVIEWER requirements.

### Changes Made

| Aspect | RFC v1.0 | Implementation v1.1 | Rationale |
|:-------|:---------|:--------------------|:----------|
| **Method Name** | `insert_batch` | `batch_insert` | Consistency with Rust ecosystem (`batch_*` prefix pattern) |
| **Signature** | `&[&[f32]]` | `I: IntoIterator<Item = (VectorId, Vec<f32>)>` | Allows caller-provided IDs, more flexible input types |
| **Storage Param** | `&mut VectorStorage` | Omitted | `HnswIndex` internally manages storage reference |
| **Progress API** | Separate method (`insert_batch_with_progress`) | Single method with `Option<F>` | Simpler API surface, DRY |
| **Return Type** | `BatchResult` type alias | `Result<Vec<u64>, BatchError>` | Direct type usage for clarity |
| **Error Struct** | `BatchError` struct with fields | `BatchError` enum with variants | Better matches Rust error handling patterns |

### Justification

1. **Generic Iterator (`IntoIterator`)**: Allows callers to use any collection type (Vec, slice, custom iterators, streams) without conversion. This is more idiomatic Rust and enables zero-copy patterns.

2. **Caller-Provided IDs**: The RFC assumed auto-generated IDs, but real-world use cases (data migration, external ID systems) benefit from explicit ID control.

3. **Storage Parameter Omission**: The `HnswIndex` already maintains a reference to its configuration. Exposing storage as a parameter creates confusion about ownership. The implementation uses internal storage coordination.

4. **Combined Progress Method**: Having `batch_insert` and `batch_insert_with_progress` as separate methods violates DRY. Using `Option<F>` keeps one entry point with optional progress.

5. **Error Enum**: An enum with distinct variants provides pattern matching support and clearer error handling than a struct with optional fields.

### Migration Path

Code written against RFC v1.0 should migrate as follows:

```rust
// RFC v1.0 style (DEPRECATED)
let ids = index.insert_batch(&vectors, &mut storage)?;

// v1.1 style (CURRENT)
let vectors_with_ids: Vec<_> = vectors.iter()
    .enumerate()
    .map(|(i, v)| (i as u64, v.to_vec()))
    .collect();
let ids = index.batch_insert(vectors_with_ids, None)?;
```

### Status

These changes are approved for implementation. The core design goals (WASM compatibility, memory budget, progress reporting, error handling) remain unchanged.

---

**Status:** APPROVED (with revisions)

**Next:** Implementation complete, submit for hostile review via `/review DAY_1_TASKS.md`

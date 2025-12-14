# Week 11 — Day 1 Tasks (Monday)

**Date:** 2025-01-13
**Focus:** Batch API Foundation — Trait Skeleton & Error Types
**Agent:** RUST_ENGINEER
**Status:** DRAFT

---

## Day Objective

Establish the foundational types for batch insert functionality: the BatchInsertable trait interface and comprehensive error handling. By end of day, the codebase must compile with the new batch module, even if implementation is stubbed.

**Success Criteria:**
- `src/batch.rs` exists with BatchInsertable trait
- `src/error.rs` contains BatchError enum with all 5 error types
- `cargo build` succeeds
- Basic trait documentation complete

---

## Theoretical Foundation

### Batch Insert Design Principles

**From RFC 0001:**
```rust
pub trait BatchInsertable {
    fn batch_insert<I, F>(
        &mut self,
        vectors: I,
        progress_callback: Option<F>,
    ) -> Result<Vec<VectorId>, BatchError>
    where
        I: IntoIterator<Item = (VectorId, Vec<f32>)>,
        F: FnMut(usize, usize);
}
```

**Key Design Decisions:**
1. **Generic Iterator:** Allows any collection type (Vec, slice, custom iterator)
2. **Optional Callback:** Progress tracking is opt-in to minimize overhead
3. **Batch Error:** Comprehensive error handling with context
4. **Return Vec<VectorId>:** Preserves insertion order for correlation

### Error Taxonomy

Based on failure mode analysis:

| Error Type | Cause | Recovery Strategy |
|:-----------|:------|:------------------|
| `DimensionMismatch` | Vector dim ≠ index dim | Reject entire batch |
| `DuplicateId` | ID already exists | Skip duplicate, continue |
| `InvalidVector` | NaN, Inf, or empty | Skip invalid, log |
| `CapacityExceeded` | Max vectors reached | Reject batch |
| `InternalError` | HNSW invariant violation | Abort, rollback |

**Atomicity Guarantee:** None. Batch insert is **best-effort**. Partial success is valid.

---

## Tasks

### W11.1: Implement BatchInsertable Trait (START)

**Priority:** P0 (Critical Path)
**Estimate:** Raw: 8h → **24h with 3x** (6h Day 1 + 18h Day 2)
**Agent:** RUST_ENGINEER
**Status:** DAY 1: Skeleton only (6h), DAY 2: Full implementation (18h)

#### Acceptance Criteria (Day 1 Subset)

- [ ] **AC1.1:** File `src/batch.rs` exists
- [ ] **AC1.2:** BatchInsertable trait declared with correct signature
- [ ] **AC1.3:** Trait has documentation explaining purpose
- [ ] **AC1.4:** `HnswIndex` implements BatchInsertable (stub returns `Ok(vec![])`)
- [ ] **AC1.5:** `cargo build` succeeds
- [ ] **AC1.6:** `cargo clippy -- -D warnings` passes

#### Implementation Specification

**File:** `src/batch.rs`

```rust,ignore
//! Batch insertion API for HNSW indexes.
//!
//! This module provides the `BatchInsertable` trait for efficient
//! insertion of multiple vectors in a single operation.

use crate::error::BatchError;
use crate::types::VectorId;

/// Trait for HNSW indexes supporting batch insertion.
///
/// # Example
///
/// ```no_run
/// use edgevec::{HnswIndex, BatchInsertable};
///
/// let mut index = HnswIndex::new(128, 16, 200, DistanceMetric::Euclidean);
/// let vectors = vec![
///     (1, vec![0.1; 128]),
///     (2, vec![0.2; 128]),
/// ];
///
/// let ids = index.batch_insert(vectors, None)?;
/// assert_eq!(ids.len(), 2);
/// # Ok::<(), BatchError>(())
/// ```
pub trait BatchInsertable {
    /// Insert multiple vectors in a single operation.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Iterator of (VectorId, Vec<f32>) pairs
    /// * `progress_callback` - Optional callback invoked as (inserted_count, total_count)
    ///
    /// # Returns
    ///
    /// Vector of successfully inserted VectorIds (may be partial on error)
    ///
    /// # Errors
    ///
    /// Returns `BatchError` if:
    /// - Any vector has wrong dimensionality
    /// - Duplicate IDs detected
    /// - Invalid vector data (NaN, Inf)
    /// - Capacity exceeded
    /// - Internal HNSW invariant violated
    fn batch_insert<I, F>(
        &mut self,
        vectors: I,
        progress_callback: Option<F>,
    ) -> Result<Vec<VectorId>, BatchError>
    where
        I: IntoIterator<Item = (VectorId, Vec<f32>)>,
        F: FnMut(usize, usize);
}
```

**File:** `src/hnsw.rs` (modification)

```rust,ignore
impl BatchInsertable for HnswIndex {
    fn batch_insert<I, F>(
        &mut self,
        _vectors: I,
        _progress_callback: Option<F>,
    ) -> Result<Vec<VectorId>, BatchError>
    where
        I: IntoIterator<Item = (VectorId, Vec<f32>)>,
        F: FnMut(usize, usize),
    {
        // DAY 1: Stub implementation
        // TODO(Day 2): Implement full batch logic
        Ok(vec![])
    }
}
```

#### Files to Create

- `src/batch.rs` (new)

#### Files to Modify

- `src/lib.rs` (add `pub mod batch;`)
- `src/hnsw.rs` (add BatchInsertable impl)

#### Dependencies

**Blocks:**
- W11.3 (Unit tests)
- W11.4 (Integration test)

**Requires:**
- W11.2 (BatchError type) [PARALLEL — can implement simultaneously]

#### Verification Commands

```bash
# Must pass before marking complete
cargo build
cargo clippy -- -D warnings
cargo doc --no-deps --open
```

---

### W11.2: Add BatchError Type

**Priority:** P0 (Critical Path)
**Estimate:** Raw: 2h → **6h with 3x**
**Agent:** RUST_ENGINEER
**Status:** COMPLETE on Day 1

#### Acceptance Criteria

- [ ] **AC2.1:** File `src/error.rs` exists
- [ ] **AC2.2:** BatchError enum has all 5 error variants
- [ ] **AC2.3:** Each variant includes context (dimension, ID, etc.)
- [ ] **AC2.4:** Implements `std::fmt::Display`
- [ ] **AC2.5:** Implements `std::error::Error`
- [ ] **AC2.6:** Error messages are human-readable
- [ ] **AC2.7:** `cargo build` succeeds

#### Implementation Specification

**File:** `src/error.rs`

```rust,ignore
//! Error types for batch operations.

use std::fmt;

/// Errors that can occur during batch insertion.
#[derive(Debug, Clone, PartialEq)]
pub enum BatchError {
    /// Vector dimensionality does not match index configuration.
    ///
    /// # Fields
    /// * `expected` - Expected dimension from index
    /// * `actual` - Actual dimension of rejected vector
    /// * `vector_id` - ID of the problematic vector
    DimensionMismatch {
        expected: usize,
        actual: usize,
        vector_id: u64,
    },

    /// Vector ID already exists in the index.
    ///
    /// # Fields
    /// * `id` - Duplicate vector ID
    DuplicateId { id: u64 },

    /// Vector contains invalid floating-point values (NaN, Infinity).
    ///
    /// # Fields
    /// * `vector_id` - ID of the invalid vector
    /// * `reason` - Description of the invalid value
    InvalidVector { vector_id: u64, reason: String },

    /// Index has reached maximum capacity.
    ///
    /// # Fields
    /// * `current` - Current number of vectors
    /// * `max` - Maximum allowed vectors
    CapacityExceeded { current: usize, max: usize },

    /// Internal HNSW invariant violated during insertion.
    ///
    /// # Fields
    /// * `message` - Description of the violated invariant
    InternalError { message: String },
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchError::DimensionMismatch {
                expected,
                actual,
                vector_id,
            } => write!(
                f,
                "Dimension mismatch for vector {}: expected {}, got {}",
                vector_id, expected, actual
            ),
            BatchError::DuplicateId { id } => {
                write!(f, "Duplicate vector ID: {}", id)
            }
            BatchError::InvalidVector { vector_id, reason } => {
                write!(f, "Invalid vector {}: {}", vector_id, reason)
            }
            BatchError::CapacityExceeded { current, max } => {
                write!(
                    f,
                    "Capacity exceeded: current={}, max={}",
                    current, max
                )
            }
            BatchError::InternalError { message } => {
                write!(f, "Internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for BatchError {}
```

#### Files to Create

- `src/error.rs` (new)

#### Files to Modify

- `src/lib.rs` (add `pub mod error;`)

#### Dependencies

**Blocks:**
- W11.1 (BatchInsertable trait)
- W11.6 (Error handling tests)

**Requires:**
- None (independent task)

#### Verification Commands

```bash
# Must pass before marking complete
cargo build
cargo clippy -- -D warnings
cargo test error --lib
```

---

## Day 1 Summary

**Total Effort:** 6h (W11.1 skeleton) + 6h (W11.2 complete) + 6h (W11.1 carryover) = **18h**

**Calculation:** Raw: 6h → 6h × 3 = 18h

**Deliverables:**
1. ✅ `src/batch.rs` with BatchInsertable trait declaration
2. ✅ `src/error.rs` with BatchError enum (all 5 variants)
3. ✅ Stub implementation in HnswIndex
4. ✅ Compiles successfully
5. ✅ Clippy passes

**Carryover to Day 2:**
- W11.1 full implementation (18h remaining)

**Blockers Removed:**
- None (W11.1 skeleton unblocks W11.3, W11.4, W11.6, W11.7)

**Status Validation:**
```bash
# Run before end of day
cargo build
cargo clippy -- -D warnings
cargo doc --no-deps
git status  # Ensure src/batch.rs and src/error.rs exist
```

---

## HOSTILE_REVIEWER Pre-Flight Checklist

Before submitting Day 1 work for review:

- [ ] All acceptance criteria met for W11.2 (complete)
- [ ] Partial acceptance criteria met for W11.1 (skeleton only)
- [ ] No compiler warnings
- [ ] Trait documentation matches RFC 0001
- [ ] Error messages are user-friendly
- [ ] Code follows Rust API guidelines
- [ ] Git commit messages reference task IDs

---

**PLANNER Notes:**
- Day 1 focuses on "make it compile" over "make it work"
- Stub implementation allows parallel test development on Day 2-3
- Error type designed for extensibility (can add variants without breaking)
- 3x multiplier accounts for API design iteration

**Status:** DRAFT
**Next:** Submit for hostile review after Day 1 completion

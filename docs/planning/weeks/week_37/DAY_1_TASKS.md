# Week 37 Day 1: Sparse Module Structure

**Date:** 2026-01-12
**Focus:** Create `src/sparse/` module skeleton with proper exports
**Estimated Duration:** 2 hours
**Phase:** RFC-007 Implementation Phase 1 (Core Types)
**Dependencies:** RFC-007 (APPROVED), SPARSE_MODULE_STRUCTURE.md

---

## Tasks

### W37.1.1: Create `src/sparse/mod.rs`

**Objective:** Define module structure with feature flag and exports.

**Rust Implementation:**

```rust
// src/sparse/mod.rs

//! Sparse vector support for hybrid search.
//!
//! This module provides sparse vector types and operations for combining
//! dense semantic embeddings with sparse keyword features (BM25, TF-IDF).
//!
//! # Feature Flags
//!
//! - `sparse` (default): Core sparse types and metrics
//!
//! # Example
//!
//! ```rust
//! use edgevec::sparse::{SparseVector, SparseError};
//!
//! // Create a sparse vector from sorted indices and values
//! let indices = vec![0, 5, 10];
//! let values = vec![0.5, 0.3, 0.2];
//! let sparse = SparseVector::new(indices, values, 100)?;
//!
//! // Compute dot product
//! let other = SparseVector::new(vec![5, 10], vec![0.4, 0.6], 100)?;
//! let dot = sparse.dot(&other);
//! # Ok::<(), SparseError>(())
//! ```

mod error;
mod vector;
mod metrics;

// Placeholders for future phases
// mod storage;  // Week 38
// mod search;   // Week 39

pub use error::SparseError;
pub use vector::SparseVector;
pub use metrics::{sparse_dot_product, sparse_cosine, sparse_norm};
```

**Acceptance Criteria:**
- [ ] Module file created at `src/sparse/mod.rs`
- [ ] Feature flag `sparse` added to `Cargo.toml` (default enabled)
- [ ] Re-exports `SparseError`, `SparseVector`
- [ ] Re-exports `sparse_dot_product`, `sparse_cosine`, `sparse_norm`
- [ ] Doc comments with example code

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W37.1.2: Create `src/sparse/error.rs`

**Objective:** Define `SparseError` enum with all RFC-007 variants.

**Rust Implementation:**

```rust
// src/sparse/error.rs

//! Error types for sparse vector operations.

use thiserror::Error;

/// Errors that can occur during sparse vector operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SparseError {
    /// Indices are not sorted in ascending order.
    #[error("indices must be sorted in ascending order")]
    UnsortedIndices,

    /// Duplicate index found at the specified position.
    #[error("duplicate index at position {0}")]
    DuplicateIndex(usize),

    /// Index exceeds the vector dimension.
    #[error("index {index} exceeds dimension {dim}")]
    IndexOutOfBounds {
        /// The invalid index value.
        index: u32,
        /// The maximum allowed dimension.
        dim: u32,
    },

    /// Value at index is NaN or Infinity.
    #[error("value at index {0} is NaN or Infinity")]
    InvalidValue(usize),

    /// Sparse vector must have at least one element.
    #[error("sparse vector must have at least one element")]
    EmptyVector,

    /// Indices and values arrays have different lengths.
    #[error("indices and values length mismatch: {indices} vs {values}")]
    LengthMismatch {
        /// Length of indices array.
        indices: usize,
        /// Length of values array.
        values: usize,
    },

    /// Sparse ID not found in storage.
    #[error("sparse ID {0} not found")]
    IdNotFound(u64),

    /// Cannot normalize a zero vector.
    #[error("cannot normalize zero vector")]
    ZeroNorm,
}
```

**Acceptance Criteria:**
- [ ] All 8 error variants from RFC-007 implemented
- [ ] Uses `thiserror` for error derivation
- [ ] All variants have descriptive error messages
- [ ] `Clone` and `PartialEq` derived for testing
- [ ] Doc comments on each variant

**Test Cases:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_unsorted() {
        let err = SparseError::UnsortedIndices;
        assert!(err.to_string().contains("sorted"));
    }

    #[test]
    fn test_error_display_duplicate() {
        let err = SparseError::DuplicateIndex(5);
        assert!(err.to_string().contains("5"));
    }

    #[test]
    fn test_error_display_out_of_bounds() {
        let err = SparseError::IndexOutOfBounds { index: 100, dim: 50 };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_error_display_invalid_value() {
        let err = SparseError::InvalidValue(3);
        assert!(err.to_string().contains("3"));
    }

    #[test]
    fn test_error_display_empty() {
        let err = SparseError::EmptyVector;
        assert!(err.to_string().contains("at least one"));
    }

    #[test]
    fn test_error_display_length_mismatch() {
        let err = SparseError::LengthMismatch { indices: 5, values: 3 };
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("3"));
    }

    #[test]
    fn test_error_display_id_not_found() {
        let err = SparseError::IdNotFound(42);
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn test_error_display_zero_norm() {
        let err = SparseError::ZeroNorm;
        assert!(err.to_string().contains("zero"));
    }
}
```

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W37.1.3: Update `src/lib.rs`

**Objective:** Add sparse module to library exports.

**Rust Implementation:**

```rust
// Add to src/lib.rs after existing module declarations

/// Sparse vector support for hybrid search.
#[cfg(feature = "sparse")]
pub mod sparse;

// Add to pub use section
#[cfg(feature = "sparse")]
pub use sparse::{SparseError, SparseVector};
```

**Cargo.toml Update:**

```toml
[features]
default = ["sparse"]
sparse = []
```

**Acceptance Criteria:**
- [ ] `sparse` module conditionally compiled with feature flag
- [ ] `SparseError` and `SparseVector` re-exported at crate root
- [ ] `cargo check --features sparse` passes
- [ ] `cargo check --no-default-features` passes (sparse disabled)

**Verification Commands:**

```bash
# With sparse feature (default)
cargo check --features sparse

# Without sparse feature
cargo check --no-default-features

# Full check with all features
cargo check --all-features
```

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W37.1.4: Create Placeholder Files

**Objective:** Create empty placeholder files for future phases.

**Files to Create:**

**`src/sparse/vector.rs`:**
```rust
// src/sparse/vector.rs

//! SparseVector type using CSR format.
//!
//! Implementation in Day 2.

use crate::sparse::error::SparseError;

/// Sparse vector using Compressed Sparse Row (CSR) format.
///
/// # Memory Layout
///
/// - `indices`: `[u32; N]` — sorted positions of non-zero elements
/// - `values`: `[f32; N]` — corresponding values
/// - `dim`: `u32` — maximum dimension (vocabulary/feature space size)
///
/// # Invariants
///
/// - Indices are sorted in ascending order
/// - No duplicate indices
/// - No NaN or Infinity in values
/// - At least one element (`nnz >= 1`)
/// - All indices `< dim`
#[derive(Clone, Debug, PartialEq)]
pub struct SparseVector {
    indices: Vec<u32>,
    values: Vec<f32>,
    dim: u32,
}

impl SparseVector {
    /// Create from pre-sorted indices and values.
    ///
    /// # Errors
    ///
    /// Returns `SparseError` if validation fails.
    pub fn new(_indices: Vec<u32>, _values: Vec<f32>, _dim: u32) -> Result<Self, SparseError> {
        todo!("Day 2: Implement SparseVector::new")
    }

    /// Number of non-zero elements.
    #[must_use]
    pub fn nnz(&self) -> usize {
        self.indices.len()
    }

    /// Get indices slice.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Get values slice.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Get dimension.
    #[must_use]
    pub fn dim(&self) -> u32 {
        self.dim
    }
}
```

**`src/sparse/metrics.rs`:**
```rust
// src/sparse/metrics.rs

//! Distance metrics for sparse vectors.
//!
//! Implementation in Day 3.

use crate::sparse::SparseVector;

/// Sparse dot product using merge-intersection.
///
/// Complexity: O(|a| + |b|) worst case.
///
/// # Arguments
///
/// * `a` - First sparse vector
/// * `b` - Second sparse vector
///
/// # Returns
///
/// The dot product of the two vectors.
#[inline]
pub fn sparse_dot_product(_a: &SparseVector, _b: &SparseVector) -> f32 {
    todo!("Day 3: Implement sparse_dot_product")
}

/// L2 norm of sparse vector.
///
/// # Arguments
///
/// * `v` - Sparse vector
///
/// # Returns
///
/// The L2 norm (Euclidean length) of the vector.
#[inline]
pub fn sparse_norm(_v: &SparseVector) -> f32 {
    todo!("Day 3: Implement sparse_norm")
}

/// Sparse cosine similarity.
///
/// Returns value in [-1, 1] for non-zero vectors, 0 for zero vectors.
///
/// # Arguments
///
/// * `a` - First sparse vector
/// * `b` - Second sparse vector
///
/// # Returns
///
/// Cosine similarity between the two vectors.
#[inline]
pub fn sparse_cosine(_a: &SparseVector, _b: &SparseVector) -> f32 {
    todo!("Day 3: Implement sparse_cosine")
}
```

**Acceptance Criteria:**
- [ ] `src/sparse/vector.rs` created with `SparseVector` stub
- [ ] `src/sparse/metrics.rs` created with metric function stubs
- [ ] All stubs have `todo!()` macros
- [ ] Doc comments describe future implementation
- [ ] `cargo check --features sparse` passes

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

## Day 1 Checklist

- [ ] W37.1.1: `src/sparse/mod.rs` created
- [ ] W37.1.2: `src/sparse/error.rs` with all 8 variants
- [ ] W37.1.3: `src/lib.rs` updated with sparse module
- [ ] W37.1.4: Placeholder files created
- [ ] Feature flag `sparse` added to `Cargo.toml`
- [ ] `cargo check --features sparse` passes
- [ ] `cargo clippy --features sparse -- -D warnings` passes
- [ ] Error unit tests pass

## Day 1 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Module compiles | `cargo check --features sparse` |
| Error types complete | 8 variants, all tested |
| Clippy clean | `cargo clippy -- -D warnings` |
| Feature flag works | `cargo check --no-default-features` |

## Day 1 Handoff

After completing Day 1:

**Artifacts Generated:**
- `src/sparse/mod.rs`
- `src/sparse/error.rs`
- `src/sparse/vector.rs` (stub)
- `src/sparse/metrics.rs` (stub)
- Updated `src/lib.rs`
- Updated `Cargo.toml`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 2 — SparseVector Implementation

---

*Agent: PLANNER + RUST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2026-01-08*

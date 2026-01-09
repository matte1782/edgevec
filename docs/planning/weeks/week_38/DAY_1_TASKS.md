# Week 38 Day 1: SparseStorage Struct Definition

**Date:** 2026-01-19
**Focus:** Create `src/sparse/storage.rs` with SparseStorage struct
**Estimated Duration:** 2 hours
**Phase:** RFC-007 Implementation Phase 2 (Storage)
**Dependencies:** Week 37 (COMPLETE)

---

## Tasks

### W38.1.1: Create `src/sparse/storage.rs` with SparseStorage Struct

**Objective:** Define the `SparseStorage` struct using packed arrays (CSR-like layout).

**From RFC-007:**
```rust
pub struct SparseStorage {
    /// Packed indices: all vectors' indices concatenated
    indices: Vec<u32>,
    /// Packed values: all vectors' values concatenated
    values: Vec<f32>,
    /// Offsets into packed arrays: offset[i] = start of vector i
    offsets: Vec<u32>,
    /// Maximum dimension for each stored vector
    dims: Vec<u32>,
    /// Deletion bitmap
    deleted: BitVec,
    /// Next ID
    next_id: u64,
}
```

**Rust Implementation:**

```rust
// src/sparse/storage.rs

//! Sparse vector storage using packed arrays.
//!
//! This module provides efficient storage for sparse vectors using a CSR-like
//! packed format. All vectors' indices and values are concatenated into
//! contiguous arrays, with offsets tracking the start of each vector.
//!
//! # Memory Layout
//!
//! For N vectors with total M non-zero elements:
//! - `indices`: `[u32; M]` - concatenated indices (4M bytes)
//! - `values`: `[f32; M]` - concatenated values (4M bytes)
//! - `offsets`: `[u32; N+1]` - start position of each vector (4(N+1) bytes)
//! - `dims`: `[u32; N]` - dimension of each vector (4N bytes)
//! - `deleted`: `BitVec` - deletion bitmap (N/8 bytes)
//!
//! # Example
//!
//! ```rust
//! use edgevec::sparse::{SparseStorage, SparseVector, SparseId};
//!
//! let mut storage = SparseStorage::new();
//!
//! let v1 = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100)?;
//! let id = storage.insert(&v1)?;
//!
//! let retrieved = storage.get(id);
//! assert!(retrieved.is_some());
//! # Ok::<(), edgevec::sparse::SparseError>(())
//! ```

use bitvec::prelude::*;
use serde::{Deserialize, Serialize};

use crate::sparse::error::SparseError;
use crate::sparse::vector::SparseVector;

/// Unique identifier for a sparse vector in storage.
///
/// This is a newtype wrapper around `u64` to provide type safety
/// and prevent accidental mixing with other ID types.
///
/// # ID Space
///
/// - IDs start from 0 and increment monotonically
/// - Deleted IDs are NOT reused (simplifies concurrency)
/// - High bit (0x8000_0000_0000_0000) reserved for future hybrid ID encoding
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SparseId(u64);

impl SparseId {
    /// Create a new SparseId from a raw u64 value.
    ///
    /// # Note
    ///
    /// This should typically only be used during deserialization.
    /// Normal ID creation happens through `SparseStorage::insert`.
    #[must_use]
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw u64 value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Get as usize for array indexing.
    ///
    /// # Panics
    ///
    /// Panics on 32-bit systems if ID exceeds usize::MAX.
    #[must_use]
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for SparseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SparseId({})", self.0)
    }
}

impl From<u64> for SparseId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<SparseId> for u64 {
    fn from(id: SparseId) -> Self {
        id.0
    }
}

/// Storage for sparse vectors using packed arrays.
///
/// This struct stores multiple sparse vectors efficiently by concatenating
/// all indices and values into contiguous arrays. An offset array tracks
/// where each vector starts.
///
/// # Memory Estimate (100k vectors, avg 50 non-zero)
///
/// ```text
/// indices: 100k * 50 * 4 bytes = 20 MB
/// values:  100k * 50 * 4 bytes = 20 MB
/// offsets: 100k * 4 bytes      = 0.4 MB
/// dims:    100k * 4 bytes      = 0.4 MB
/// deleted: 100k / 8 bytes      = 12.5 KB
/// ----------------------------------------
/// Total: ~41 MB
/// ```
///
/// # Invariants
///
/// - `offsets.len() == dims.len() + 1` (offsets has sentinel at end)
/// - `offsets[offsets.len() - 1] == indices.len() == values.len()`
/// - `deleted.len() == dims.len()`
/// - For valid ID `i`: `offsets[i] <= offsets[i+1]`
/// - All indices within a vector range are sorted
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SparseStorage {
    /// Packed indices: all vectors' indices concatenated.
    indices: Vec<u32>,

    /// Packed values: all vectors' values concatenated.
    values: Vec<f32>,

    /// Offsets into packed arrays: offsets[i] = start of vector i.
    /// Length is N+1 where N is number of vectors (sentinel at end).
    offsets: Vec<u32>,

    /// Maximum dimension for each stored vector.
    dims: Vec<u32>,

    /// Deletion bitmap: deleted[i] = true if vector i is deleted.
    deleted: BitVec,

    /// Next ID to assign.
    next_id: u64,
}
```

**Acceptance Criteria:**
- [ ] File created at `src/sparse/storage.rs`
- [ ] `SparseId` newtype defined with `from_raw`, `as_u64`, `as_usize`
- [ ] `SparseStorage` struct defined with all 6 fields from RFC-007
- [ ] Derives `Clone`, `Debug`, `Serialize`, `Deserialize`
- [ ] Doc comments with memory estimates and invariants
- [ ] Example code in module doc comment

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W38.1.2: Add SparseId Newtype Wrapper

**Objective:** Ensure `SparseId` has complete implementation with all trait impls.

**Rust Implementation (verification of W38.1.1):**

```rust
// Included in W38.1.1 above - verification checklist:

// Required traits:
// - Clone, Copy (value type)
// - Debug (debugging)
// - PartialEq, Eq (comparison)
// - Hash (use in HashMaps)
// - Serialize, Deserialize (persistence)
// - Display (user-facing output)
// - From<u64>, From<SparseId> for u64 (conversions)

// Required methods:
// - from_raw(u64) -> Self
// - as_u64(self) -> u64
// - as_usize(self) -> usize
```

**Test Cases:**

```rust
#[cfg(test)]
mod sparse_id_tests {
    use super::*;

    #[test]
    fn test_sparse_id_from_raw() {
        let id = SparseId::from_raw(42);
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn test_sparse_id_as_usize() {
        let id = SparseId::from_raw(100);
        assert_eq!(id.as_usize(), 100);
    }

    #[test]
    fn test_sparse_id_from_u64() {
        let id: SparseId = 42u64.into();
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn test_sparse_id_into_u64() {
        let id = SparseId::from_raw(42);
        let raw: u64 = id.into();
        assert_eq!(raw, 42);
    }

    #[test]
    fn test_sparse_id_display() {
        let id = SparseId::from_raw(42);
        assert_eq!(format!("{}", id), "SparseId(42)");
    }

    #[test]
    fn test_sparse_id_debug() {
        let id = SparseId::from_raw(42);
        assert_eq!(format!("{:?}", id), "SparseId(42)");
    }

    #[test]
    fn test_sparse_id_equality() {
        let id1 = SparseId::from_raw(42);
        let id2 = SparseId::from_raw(42);
        let id3 = SparseId::from_raw(43);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_sparse_id_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(SparseId::from_raw(1));
        set.insert(SparseId::from_raw(2));
        set.insert(SparseId::from_raw(1)); // duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_sparse_id_copy() {
        let id1 = SparseId::from_raw(42);
        let id2 = id1; // Copy, not move
        assert_eq!(id1, id2); // id1 still valid
    }
}
```

**Acceptance Criteria:**
- [ ] `SparseId` implements all required traits
- [ ] All conversion methods implemented
- [ ] Unit tests for all SparseId functionality
- [ ] Doc comments on all public methods

**Estimated Duration:** 15 minutes (mostly verification)

**Agent:** RUST_ENGINEER

---

### W38.1.3: Implement `new()` Constructor

**Objective:** Implement `SparseStorage::new()` constructor with proper initialization.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

impl SparseStorage {
    /// Create a new empty sparse storage.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseStorage;
    ///
    /// let storage = SparseStorage::new();
    /// assert_eq!(storage.len(), 0);
    /// assert!(storage.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            values: Vec::new(),
            offsets: vec![0], // Sentinel: first vector starts at 0
            dims: Vec::new(),
            deleted: BitVec::new(),
            next_id: 0,
        }
    }

    /// Create a new sparse storage with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `vector_capacity` - Expected number of vectors
    /// * `element_capacity` - Expected total number of non-zero elements
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseStorage;
    ///
    /// // Pre-allocate for 1000 vectors with ~50 elements each
    /// let storage = SparseStorage::with_capacity(1000, 50_000);
    /// ```
    #[must_use]
    pub fn with_capacity(vector_capacity: usize, element_capacity: usize) -> Self {
        let mut offsets = Vec::with_capacity(vector_capacity + 1);
        offsets.push(0); // Sentinel

        Self {
            indices: Vec::with_capacity(element_capacity),
            values: Vec::with_capacity(element_capacity),
            offsets,
            dims: Vec::with_capacity(vector_capacity),
            deleted: BitVec::with_capacity(vector_capacity),
            next_id: 0,
        }
    }

    /// Returns the number of vectors in storage (including deleted).
    ///
    /// For the count of non-deleted vectors, use `active_len()`.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.dims.len()
    }

    /// Returns true if storage contains no vectors.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dims.is_empty()
    }

    /// Returns the number of non-deleted vectors.
    #[must_use]
    pub fn active_len(&self) -> usize {
        self.len() - self.deleted.count_ones()
    }

    /// Returns the total number of non-zero elements across all vectors.
    #[must_use]
    #[inline]
    pub fn total_elements(&self) -> usize {
        self.indices.len()
    }

    /// Returns the next ID that will be assigned.
    #[must_use]
    #[inline]
    pub fn next_id(&self) -> u64 {
        self.next_id
    }
}

impl Default for SparseStorage {
    fn default() -> Self {
        Self::new()
    }
}
```

**Acceptance Criteria:**
- [ ] `new()` creates empty storage with sentinel offset
- [ ] `with_capacity()` pre-allocates all internal vectors
- [ ] `len()` returns total vector count
- [ ] `is_empty()` returns true for empty storage
- [ ] `active_len()` returns non-deleted count
- [ ] `total_elements()` returns total nnz
- [ ] `Default` trait implemented
- [ ] All methods have `#[must_use]` where appropriate

**Test Cases:**

```rust
#[cfg(test)]
mod storage_tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let storage = SparseStorage::new();

        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
        assert_eq!(storage.active_len(), 0);
        assert_eq!(storage.total_elements(), 0);
        assert_eq!(storage.next_id(), 0);
    }

    #[test]
    fn test_new_offsets_sentinel() {
        let storage = SparseStorage::new();

        // Offsets should have sentinel value 0
        assert_eq!(storage.offsets.len(), 1);
        assert_eq!(storage.offsets[0], 0);
    }

    #[test]
    fn test_with_capacity() {
        let storage = SparseStorage::with_capacity(1000, 50_000);

        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
        // Capacity should be reserved but not visible in len()
    }

    #[test]
    fn test_default() {
        let storage = SparseStorage::default();

        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }
}
```

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W38.1.4: Add Module Export in `src/sparse/mod.rs`

**Objective:** Export `SparseStorage` and `SparseId` from the sparse module.

**Rust Implementation:**

```rust
// src/sparse/mod.rs - UPDATE

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
//! use edgevec::sparse::{SparseVector, SparseStorage, SparseId, SparseError};
//!
//! // Create a sparse vector from sorted indices and values
//! let indices = vec![0, 5, 10];
//! let values = vec![0.5, 0.3, 0.2];
//! let sparse = SparseVector::new(indices, values, 100)?;
//!
//! // Store it
//! let mut storage = SparseStorage::new();
//! let id: SparseId = storage.insert(&sparse)?;
//!
//! // Retrieve it
//! let retrieved = storage.get(id);
//! assert!(retrieved.is_some());
//! # Ok::<(), SparseError>(())
//! ```

mod error;
mod metrics;
mod storage;  // NEW: Week 38
mod vector;

// Placeholders for future phases
// mod search;   // Week 39

pub use error::SparseError;
pub use metrics::{sparse_cosine, sparse_dot_product, sparse_norm};
pub use storage::{SparseId, SparseStorage};  // NEW: Week 38
pub use vector::SparseVector;
```

**Acceptance Criteria:**
- [ ] `mod storage;` added (uncommented)
- [ ] `SparseId` re-exported
- [ ] `SparseStorage` re-exported
- [ ] Module doc example updated to include storage usage
- [ ] `cargo check --features sparse` passes

**Verification Commands:**

```bash
# Check compilation
cargo check --features sparse

# Run clippy
cargo clippy --features sparse -- -D warnings

# Run tests
cargo test --features sparse sparse_id_tests
cargo test --features sparse storage_tests

# Verify exports work from crate root
cargo check --features sparse --example sparse_storage_check
```

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

## Day 1 Checklist

- [ ] W38.1.1: `src/sparse/storage.rs` created with `SparseStorage` struct
- [ ] W38.1.2: `SparseId` newtype with all trait implementations
- [ ] W38.1.3: `new()` and `with_capacity()` constructors
- [ ] W38.1.4: Module exports updated in `src/sparse/mod.rs`
- [ ] `cargo check --features sparse` passes
- [ ] `cargo clippy --features sparse -- -D warnings` passes
- [ ] All unit tests pass

## Day 1 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Storage struct compiles | `cargo check --features sparse` |
| SparseId fully implemented | 8 unit tests pass |
| Constructors work | 4 unit tests pass |
| Clippy clean | `cargo clippy -- -D warnings` |
| Module exports correct | Can import `SparseStorage`, `SparseId` |

## Day 1 Handoff

After completing Day 1:

**Artifacts Generated:**
- `src/sparse/storage.rs` (struct definition, SparseId, constructors)
- Updated `src/sparse/mod.rs` (exports)
- Unit tests in `src/sparse/storage.rs`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 2 - SparseStorage `insert()` and `get()` Operations

---

## Memory Layout Diagram

```
SparseStorage with 3 vectors:
  Vector 0: indices=[1,5], values=[0.1, 0.2], dim=100
  Vector 1: indices=[0,3,7], values=[0.3, 0.4, 0.5], dim=100
  Vector 2: indices=[2], values=[0.6], dim=50

Internal Layout:
  indices: [1, 5, | 0, 3, 7, | 2]
           ^      ^          ^
           |      |          |
  offsets: [0,    2,         5,    6]
            |     |          |     |
            v0    v1         v2    sentinel

  values:  [0.1, 0.2, | 0.3, 0.4, 0.5, | 0.6]
  dims:    [100, 100, 50]
  deleted: [0,   0,   0]
  next_id: 3
```

---

*Agent: PLANNER*
*Status: [PROPOSED]*
*Date: 2026-01-09*

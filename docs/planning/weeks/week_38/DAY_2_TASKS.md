# Week 38 Day 2: SparseStorage Insert Operation

**Date:** 2026-01-20
**Focus:** Implement `insert()` method for SparseStorage
**Estimated Duration:** 3 hours
**Phase:** RFC-007 Implementation Phase 2 (Storage)
**Dependencies:** Week 38 Day 1 (SparseStorage struct + SparseId type)

---

## Context

Day 1 established the `SparseStorage` struct with packed arrays and the `SparseId` type. Day 2 implements the core insert operation that appends vectors to storage.

**From RFC-007 SparseStorage Design:**
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

**Performance Target (RFC-007):** Insert < 50us P50, < 100us P99

---

## Tasks

### W38.2.1: Implement `insert()` Method

**Objective:** Add `insert(&mut self, vector: &SparseVector) -> Result<SparseId, SparseError>` to SparseStorage.

**Rust Implementation:**

```rust
// src/sparse/storage.rs

impl SparseStorage {
    /// Insert a sparse vector into storage.
    ///
    /// Appends the vector's indices and values to the packed arrays,
    /// records the offset, and returns a unique `SparseId`.
    ///
    /// # Arguments
    ///
    /// * `vector` - The sparse vector to insert
    ///
    /// # Returns
    ///
    /// * `Ok(SparseId)` - The unique identifier for the inserted vector
    /// * `Err(SparseError)` - If validation fails (e.g., empty vector)
    ///
    /// # Performance
    ///
    /// Target: < 50us P50, < 100us P99
    /// Complexity: O(nnz) where nnz is the number of non-zero elements
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let vector = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100)?;
    /// let id = storage.insert(&vector)?;
    ///
    /// assert_eq!(storage.len(), 1);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn insert(&mut self, vector: &SparseVector) -> Result<SparseId, SparseError> {
        // Validation: nnz > 0 is already enforced by SparseVector invariants.
        // SparseVector cannot be constructed with zero elements, so we rely on
        // that guarantee. However, we perform a defensive check anyway.
        if vector.nnz() == 0 {
            return Err(SparseError::EmptyVector);
        }

        // Record offset (current end of packed arrays)
        let offset = self.indices.len() as u32;
        self.offsets.push(offset);

        // Append indices and values to packed arrays
        self.indices.extend_from_slice(vector.indices());
        self.values.extend_from_slice(vector.values());

        // Store dimension for this vector
        self.dims.push(vector.dim());

        // Extend deletion bitmap (not deleted)
        self.deleted.push(false);

        // Allocate ID and increment counter
        let id = SparseId(self.next_id);
        self.next_id += 1;

        Ok(id)
    }
}
```

**Acceptance Criteria:**
- [ ] `insert()` method implemented on `SparseStorage`
- [ ] Appends indices to packed `indices` array
- [ ] Appends values to packed `values` array
- [ ] Records offset in `offsets` array
- [ ] Stores dimension in `dims` array
- [ ] Extends `deleted` bitmap with `false`
- [ ] Returns unique `SparseId`
- [ ] Increments `next_id`
- [ ] Returns `SparseError::EmptyVector` for empty vector (defensive)

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W38.2.2: Implement Validation Logic

**Objective:** Ensure insert validates the vector properly. Dimension consistency is optional (each vector can have different dim), but nnz > 0 is required.

**Rust Implementation:**

```rust
// src/sparse/storage.rs

impl SparseStorage {
    /// Validate a vector before insertion.
    ///
    /// # Validation Rules
    ///
    /// 1. `nnz > 0` - Vector must have at least one element
    ///    (This is guaranteed by SparseVector invariants, but we check defensively)
    ///
    /// 2. Dimension consistency is NOT enforced - each vector can have
    ///    different dimensions (vocabulary sizes). This supports:
    ///    - Multi-tenant scenarios with different vocabularies
    ///    - Incremental vocabulary growth
    ///    - Mixed feature spaces
    ///
    /// # Arguments
    ///
    /// * `vector` - The sparse vector to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Validation passed
    /// * `Err(SparseError)` - Validation failed
    fn validate_for_insert(&self, vector: &SparseVector) -> Result<(), SparseError> {
        // Check nnz > 0 (defensive - SparseVector already guarantees this)
        if vector.nnz() == 0 {
            return Err(SparseError::EmptyVector);
        }

        // NOTE: We intentionally do NOT enforce dimension consistency.
        // Different vectors can have different dimensions (vocabulary sizes).
        // This is a deliberate design choice per RFC-007 discussion.
        //
        // If dimension enforcement is needed in the future, uncomment:
        // if let Some(expected_dim) = self.expected_dim {
        //     if vector.dim() != expected_dim {
        //         return Err(SparseError::DimensionMismatch { ... });
        //     }
        // }

        Ok(())
    }

    /// Insert with explicit validation step.
    ///
    /// This is the recommended entry point that separates validation from insertion.
    pub fn insert(&mut self, vector: &SparseVector) -> Result<SparseId, SparseError> {
        // Step 1: Validate
        self.validate_for_insert(vector)?;

        // Step 2: Record offset (current end of packed arrays)
        let offset = self.indices.len() as u32;
        self.offsets.push(offset);

        // Step 3: Append indices and values to packed arrays
        self.indices.extend_from_slice(vector.indices());
        self.values.extend_from_slice(vector.values());

        // Step 4: Store dimension for this vector
        self.dims.push(vector.dim());

        // Step 5: Extend deletion bitmap (not deleted)
        self.deleted.push(false);

        // Step 6: Allocate ID and increment counter
        let id = SparseId(self.next_id);
        self.next_id += 1;

        Ok(id)
    }
}
```

**Acceptance Criteria:**
- [ ] `validate_for_insert()` helper method implemented
- [ ] Returns `SparseError::EmptyVector` for nnz == 0
- [ ] Does NOT enforce dimension consistency (intentional)
- [ ] Validation is called before any mutation
- [ ] Clear documentation explaining dimension flexibility

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W38.2.3: Add Unit Tests for Insert

**Objective:** Comprehensive test coverage for the insert operation.

**Rust Implementation:**

```rust
// src/sparse/storage.rs - tests module

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::SparseVector;

    // ============= Insert Basic Tests =============

    #[test]
    fn test_insert_single_vector() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();

        let id = storage.insert(&vector).unwrap();

        assert_eq!(id.0, 0); // First ID is 0
        assert_eq!(storage.len(), 1);
        assert!(!storage.is_empty());
    }

    #[test]
    fn test_insert_multiple_vectors() {
        let mut storage = SparseStorage::new();

        let v1 = SparseVector::new(vec![0, 5], vec![0.1, 0.2], 100).unwrap();
        let v2 = SparseVector::new(vec![10, 20, 30], vec![0.3, 0.4, 0.5], 100).unwrap();
        let v3 = SparseVector::singleton(42, 1.0, 100).unwrap();

        let id1 = storage.insert(&v1).unwrap();
        let id2 = storage.insert(&v2).unwrap();
        let id3 = storage.insert(&v3).unwrap();

        assert_eq!(id1.0, 0);
        assert_eq!(id2.0, 1);
        assert_eq!(id3.0, 2);
        assert_eq!(storage.len(), 3);
    }

    #[test]
    fn test_insert_preserves_order() {
        let mut storage = SparseStorage::new();

        // Insert vectors with known values
        let v1 = SparseVector::new(vec![0, 1], vec![1.0, 2.0], 10).unwrap();
        let v2 = SparseVector::new(vec![2, 3, 4], vec![3.0, 4.0, 5.0], 10).unwrap();

        storage.insert(&v1).unwrap();
        storage.insert(&v2).unwrap();

        // Verify packed arrays contain concatenated data
        assert_eq!(storage.indices_slice(), &[0, 1, 2, 3, 4]);
        assert_eq!(storage.values_slice(), &[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(storage.offsets_slice(), &[0, 2]); // v1 starts at 0, v2 at 2
    }

    #[test]
    fn test_insert_records_dimensions() {
        let mut storage = SparseStorage::new();

        let v1 = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let v2 = SparseVector::new(vec![0], vec![1.0], 500).unwrap(); // Different dim
        let v3 = SparseVector::new(vec![0], vec![1.0], 10000).unwrap(); // Different dim

        storage.insert(&v1).unwrap();
        storage.insert(&v2).unwrap();
        storage.insert(&v3).unwrap();

        assert_eq!(storage.dims_slice(), &[100, 500, 10000]);
    }

    #[test]
    fn test_insert_updates_deleted_bitmap() {
        let mut storage = SparseStorage::new();

        let v1 = SparseVector::singleton(0, 1.0, 10).unwrap();
        let v2 = SparseVector::singleton(1, 2.0, 10).unwrap();

        let id1 = storage.insert(&v1).unwrap();
        let id2 = storage.insert(&v2).unwrap();

        // Both vectors should not be deleted
        assert!(!storage.is_deleted(id1));
        assert!(!storage.is_deleted(id2));
    }

    // ============= Insert with Different Dimensions =============

    #[test]
    fn test_insert_allows_different_dimensions() {
        let mut storage = SparseStorage::new();

        // Different dimension vectors - should all succeed
        let v1 = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let v2 = SparseVector::new(vec![0], vec![1.0], 1000).unwrap();
        let v3 = SparseVector::new(vec![0], vec![1.0], 50000).unwrap();

        assert!(storage.insert(&v1).is_ok());
        assert!(storage.insert(&v2).is_ok());
        assert!(storage.insert(&v3).is_ok());
    }

    // ============= Insert ID Sequence Tests =============

    #[test]
    fn test_insert_id_monotonic() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::singleton(0, 1.0, 10).unwrap();

        let mut prev_id = storage.insert(&vector).unwrap();
        for _ in 0..100 {
            let current_id = storage.insert(&vector).unwrap();
            assert!(current_id.0 > prev_id.0);
            prev_id = current_id;
        }
    }

    #[test]
    fn test_insert_id_starts_from_zero() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::singleton(0, 1.0, 10).unwrap();

        let first_id = storage.insert(&vector).unwrap();
        assert_eq!(first_id.0, 0);
    }

    // ============= Insert Capacity Tests =============

    #[test]
    fn test_insert_many_vectors() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::new(vec![0, 1, 2], vec![0.1, 0.2, 0.3], 100).unwrap();

        // Insert 1000 vectors
        for i in 0..1000 {
            let id = storage.insert(&vector).unwrap();
            assert_eq!(id.0, i);
        }

        assert_eq!(storage.len(), 1000);
        // 1000 vectors * 3 elements each = 3000 total elements
        assert_eq!(storage.indices_slice().len(), 3000);
        assert_eq!(storage.values_slice().len(), 3000);
    }

    // ============= Insert with Singleton =============

    #[test]
    fn test_insert_singleton() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::singleton(42, 0.5, 100).unwrap();

        let id = storage.insert(&vector).unwrap();

        assert_eq!(storage.len(), 1);
        assert_eq!(storage.indices_slice(), &[42]);
        assert_eq!(storage.values_slice(), &[0.5]);
        assert_eq!(storage.offsets_slice(), &[0]);
    }

    // ============= Validation Tests =============

    #[test]
    fn test_validate_for_insert_accepts_valid() {
        let storage = SparseStorage::new();
        let vector = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();

        assert!(storage.validate_for_insert(&vector).is_ok());
    }

    // NOTE: We cannot easily test empty vector rejection at insert level
    // because SparseVector::new() already prevents empty vectors.
    // The validation is defensive for any future changes.
}
```

**Acceptance Criteria:**
- [ ] `test_insert_single_vector` - Basic insert works
- [ ] `test_insert_multiple_vectors` - Sequential inserts work
- [ ] `test_insert_preserves_order` - Packed arrays are correct
- [ ] `test_insert_records_dimensions` - Dims array populated
- [ ] `test_insert_updates_deleted_bitmap` - Bitmap extended
- [ ] `test_insert_allows_different_dimensions` - No dim enforcement
- [ ] `test_insert_id_monotonic` - IDs always increase
- [ ] `test_insert_id_starts_from_zero` - First ID is 0
- [ ] `test_insert_many_vectors` - Handles 1000 vectors
- [ ] `test_insert_singleton` - Single-element vectors work
- [ ] All tests pass: `cargo test sparse::storage`

**Estimated Duration:** 1 hour

**Agent:** RUST_ENGINEER + TEST_ENGINEER

---

### W38.2.4: Add `len()` and `is_empty()` Methods

**Objective:** Add utility methods for checking storage size.

**Rust Implementation:**

```rust
// src/sparse/storage.rs

impl SparseStorage {
    /// Returns the number of vectors in storage (including deleted).
    ///
    /// This is the total count of vectors ever inserted minus those
    /// that have been compacted away. Deleted vectors still count
    /// until compaction occurs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// assert_eq!(storage.len(), 0);
    ///
    /// let v = SparseVector::singleton(0, 1.0, 10)?;
    /// storage.insert(&v)?;
    /// assert_eq!(storage.len(), 1);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    /// Returns `true` if storage contains no vectors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// assert!(storage.is_empty());
    ///
    /// let v = SparseVector::singleton(0, 1.0, 10)?;
    /// storage.insert(&v)?;
    /// assert!(!storage.is_empty());
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// Returns the number of non-deleted vectors.
    ///
    /// This is the "active" count - vectors that can be retrieved
    /// and searched. Deleted vectors are excluded.
    ///
    /// # Performance
    ///
    /// O(n) where n is the total number of vectors (iterates bitmap).
    /// Consider caching if called frequently.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 10)?;
    /// let id = storage.insert(&v)?;
    ///
    /// assert_eq!(storage.active_count(), 1);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.deleted.iter().filter(|&deleted| !deleted).count()
    }

    /// Returns the total number of elements across all vectors.
    ///
    /// This is the sum of `nnz` for all stored vectors, useful for
    /// memory estimation and debugging.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    ///
    /// let v1 = SparseVector::new(vec![0, 1], vec![0.1, 0.2], 10)?;  // nnz = 2
    /// let v2 = SparseVector::new(vec![0, 1, 2], vec![0.1, 0.2, 0.3], 10)?;  // nnz = 3
    ///
    /// storage.insert(&v1)?;
    /// storage.insert(&v2)?;
    ///
    /// assert_eq!(storage.total_elements(), 5);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn total_elements(&self) -> usize {
        self.indices.len()
    }

    // ============= Internal Accessors for Testing =============

    /// Get slice of packed indices (for testing).
    #[cfg(test)]
    pub(crate) fn indices_slice(&self) -> &[u32] {
        &self.indices
    }

    /// Get slice of packed values (for testing).
    #[cfg(test)]
    pub(crate) fn values_slice(&self) -> &[f32] {
        &self.values
    }

    /// Get slice of offsets (for testing).
    #[cfg(test)]
    pub(crate) fn offsets_slice(&self) -> &[u32] {
        &self.offsets
    }

    /// Get slice of dimensions (for testing).
    #[cfg(test)]
    pub(crate) fn dims_slice(&self) -> &[u32] {
        &self.dims
    }

    /// Check if a vector is deleted (for testing).
    #[cfg(test)]
    pub(crate) fn is_deleted(&self, id: SparseId) -> bool {
        let idx = id.0 as usize;
        if idx >= self.deleted.len() {
            return true; // Out of bounds = deleted/invalid
        }
        self.deleted[idx]
    }
}
```

**Unit Tests for Utility Methods:**

```rust
#[cfg(test)]
mod utility_tests {
    use super::*;

    #[test]
    fn test_len_empty() {
        let storage = SparseStorage::new();
        assert_eq!(storage.len(), 0);
    }

    #[test]
    fn test_len_with_vectors() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 10).unwrap();

        storage.insert(&v).unwrap();
        assert_eq!(storage.len(), 1);

        storage.insert(&v).unwrap();
        assert_eq!(storage.len(), 2);

        storage.insert(&v).unwrap();
        assert_eq!(storage.len(), 3);
    }

    #[test]
    fn test_is_empty_true() {
        let storage = SparseStorage::new();
        assert!(storage.is_empty());
    }

    #[test]
    fn test_is_empty_false() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 10).unwrap();

        storage.insert(&v).unwrap();
        assert!(!storage.is_empty());
    }

    #[test]
    fn test_active_count_all_active() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 10).unwrap();

        storage.insert(&v).unwrap();
        storage.insert(&v).unwrap();
        storage.insert(&v).unwrap();

        assert_eq!(storage.active_count(), 3);
        assert_eq!(storage.len(), 3);
    }

    #[test]
    fn test_total_elements() {
        let mut storage = SparseStorage::new();

        let v1 = SparseVector::new(vec![0, 1], vec![0.1, 0.2], 10).unwrap(); // 2 elements
        let v2 = SparseVector::new(vec![0, 1, 2], vec![0.1, 0.2, 0.3], 10).unwrap(); // 3 elements

        storage.insert(&v1).unwrap();
        assert_eq!(storage.total_elements(), 2);

        storage.insert(&v2).unwrap();
        assert_eq!(storage.total_elements(), 5);
    }

    #[test]
    fn test_total_elements_empty() {
        let storage = SparseStorage::new();
        assert_eq!(storage.total_elements(), 0);
    }
}
```

**Acceptance Criteria:**
- [ ] `len()` returns number of vectors (including deleted)
- [ ] `is_empty()` returns true for empty storage
- [ ] `active_count()` returns non-deleted count
- [ ] `total_elements()` returns sum of all nnz values
- [ ] All methods have `#[must_use]` and `#[inline]` where appropriate
- [ ] Test accessors are `#[cfg(test)]` gated
- [ ] All tests pass

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

## Day 2 Checklist

- [ ] W38.2.1: `insert()` method implemented
- [ ] W38.2.2: Validation logic with `validate_for_insert()`
- [ ] W38.2.3: All unit tests pass (14+ tests)
- [ ] W38.2.4: `len()`, `is_empty()`, `active_count()`, `total_elements()` implemented
- [ ] `cargo check --features sparse` passes
- [ ] `cargo clippy --features sparse -- -D warnings` passes
- [ ] `cargo test sparse::storage` passes

---

## Day 2 Exit Criteria

| Criterion | Verification Command |
|:----------|:--------------------|
| Insert compiles | `cargo check --features sparse` |
| Insert works | `cargo test test_insert_single_vector` |
| Multiple inserts work | `cargo test test_insert_multiple_vectors` |
| IDs are sequential | `cargo test test_insert_id_monotonic` |
| Len/Empty work | `cargo test utility_tests` |
| Clippy clean | `cargo clippy --features sparse -- -D warnings` |
| All storage tests pass | `cargo test sparse::storage` |

---

## Verification Commands

```bash
# Check compilation
cargo check --features sparse

# Run all sparse storage tests
cargo test sparse::storage --features sparse

# Run specific insert tests
cargo test test_insert --features sparse

# Run utility method tests
cargo test utility_tests --features sparse

# Check for warnings
cargo clippy --features sparse -- -D warnings

# Format code
cargo fmt

# Full test suite
cargo test --features sparse
```

---

## Day 2 Handoff

After completing Day 2:

**Artifacts Generated:**
- Updated `src/sparse/storage.rs` with:
  - `insert(&mut self, vector: &SparseVector) -> Result<SparseId, SparseError>`
  - `validate_for_insert(&self, vector: &SparseVector) -> Result<(), SparseError>`
  - `len(&self) -> usize`
  - `is_empty(&self) -> bool`
  - `active_count(&self) -> usize`
  - `total_elements(&self) -> usize`
  - Test accessor methods (cfg(test) gated)
  - 14+ unit tests for insert and utility methods

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 3 - Implement `get()` method for retrieving vectors by ID

---

## Notes for Implementation

### Why No Dimension Enforcement?

Per RFC-007 discussion, dimension consistency is intentionally NOT enforced:

1. **Multi-tenant support:** Different vocabularies per tenant
2. **Vocabulary growth:** New terms can be added incrementally
3. **Mixed feature spaces:** Combine different feature extractors
4. **WASM constraints:** Users compute sparse vectors externally

Each vector stores its own `dim`, allowing heterogeneous collections.

### Memory Layout After Insert

After inserting `v1 = [0.1@0, 0.2@5]` and `v2 = [0.3@10, 0.4@20, 0.5@30]`:

```
indices: [0, 5, 10, 20, 30]
values:  [0.1, 0.2, 0.3, 0.4, 0.5]
offsets: [0, 2]  // v1 starts at 0, v2 starts at 2
dims:    [100, 100]
deleted: [false, false]
next_id: 2
```

This packed layout minimizes memory overhead and enables efficient iteration.

---

*Agent: PLANNER*
*Status: [PROPOSED]*
*Date: 2026-01-09*

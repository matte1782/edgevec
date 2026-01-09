# Week 38 Day 5: Deletion Support for SparseStorage

**Date:** 2026-01-23
**Focus:** Implement soft delete for SparseStorage
**Estimated Duration:** 3 hours
**Phase:** RFC-007 Implementation Phase 2 (Storage)
**Dependencies:** Day 4 (SparseStorage insert/get) - MUST BE COMPLETE

---

## Context

Day 5 adds deletion support to `SparseStorage`. Following EdgeVec's established pattern from dense vector storage, we use **soft delete** with a `BitVec` deletion bitmap. This approach:

1. Avoids expensive array compaction
2. Enables fast delete operations (O(1))
3. Maintains ID stability (no ID reuse until compaction)
4. Matches the dense storage pattern (consistency)

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

---

## Tasks

### W38.5.1: Implement `delete(&mut self, id: SparseId) -> Result<bool, SparseError>`

**Objective:** Add soft delete capability to SparseStorage.

**Rust Implementation:**

```rust
// src/sparse/storage.rs

use bitvec::prelude::*;

/// Unique identifier for sparse vectors in storage.
/// Maps directly to index in offset/dims arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SparseId(pub u64);

impl SparseStorage {
    /// Delete a sparse vector by ID (soft delete).
    ///
    /// Sets the deletion flag in the bitmap. The vector data remains
    /// in memory until compaction (future phase).
    ///
    /// # Arguments
    /// * `id` - The SparseId of the vector to delete
    ///
    /// # Returns
    /// * `Ok(true)` - Vector was deleted
    /// * `Ok(false)` - Vector was already deleted
    /// * `Err(SparseError::IdNotFound)` - ID does not exist
    ///
    /// # Example
    /// ```rust
    /// let id = storage.insert(&sparse_vec)?;
    /// assert!(storage.get(id).is_some());
    ///
    /// let deleted = storage.delete(id)?;
    /// assert!(deleted);  // First delete returns true
    ///
    /// assert!(storage.get(id).is_none());  // get returns None after delete
    ///
    /// let deleted_again = storage.delete(id)?;
    /// assert!(!deleted_again);  // Second delete returns false
    /// ```
    pub fn delete(&mut self, id: SparseId) -> Result<bool, SparseError> {
        let index = id.0 as usize;

        // Check if ID exists (index must be within bounds)
        if index >= self.offsets.len() {
            return Err(SparseError::IdNotFound(id.0));
        }

        // Check if already deleted
        if self.deleted[index] {
            return Ok(false);
        }

        // Set deletion flag
        self.deleted.set(index, true);

        Ok(true)
    }
}
```

**Acceptance Criteria:**
- [ ] `delete` returns `Ok(true)` on first deletion
- [ ] `delete` returns `Ok(false)` on subsequent deletions
- [ ] `delete` returns `Err(IdNotFound)` for non-existent IDs
- [ ] Deletion is O(1) time complexity
- [ ] Does not modify indices/values arrays (soft delete only)

**Estimated Duration:** 20 minutes

**Agent:** RUST_ENGINEER

---

### W38.5.2: Implement `is_deleted(&self, id: SparseId) -> bool`

**Objective:** Query deletion status of a vector.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

impl SparseStorage {
    /// Check if a sparse vector has been deleted.
    ///
    /// # Arguments
    /// * `id` - The SparseId to check
    ///
    /// # Returns
    /// * `true` - Vector is deleted OR ID does not exist
    /// * `false` - Vector exists and is not deleted
    ///
    /// # Note
    /// Returns `true` for non-existent IDs to simplify caller logic.
    /// Use `exists()` if you need to distinguish between deleted and non-existent.
    ///
    /// # Example
    /// ```rust
    /// let id = storage.insert(&sparse_vec)?;
    /// assert!(!storage.is_deleted(id));
    ///
    /// storage.delete(id)?;
    /// assert!(storage.is_deleted(id));
    ///
    /// // Non-existent IDs also return true
    /// assert!(storage.is_deleted(SparseId(99999)));
    /// ```
    pub fn is_deleted(&self, id: SparseId) -> bool {
        let index = id.0 as usize;

        // Out of bounds = treat as deleted (doesn't exist)
        if index >= self.offsets.len() {
            return true;
        }

        self.deleted[index]
    }

    /// Check if a sparse vector ID exists (regardless of deletion status).
    ///
    /// # Returns
    /// * `true` - ID is within valid range
    /// * `false` - ID is out of range
    pub fn exists(&self, id: SparseId) -> bool {
        (id.0 as usize) < self.offsets.len()
    }
}
```

**Acceptance Criteria:**
- [ ] `is_deleted` returns `false` for active vectors
- [ ] `is_deleted` returns `true` for deleted vectors
- [ ] `is_deleted` returns `true` for non-existent IDs
- [ ] `exists` distinguishes between deleted and non-existent
- [ ] O(1) time complexity

**Estimated Duration:** 15 minutes

**Agent:** RUST_ENGINEER

---

### W38.5.3: Update `get()` to Return None for Deleted Vectors

**Objective:** Modify `get()` to respect deletion bitmap.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (update existing get method)

impl SparseStorage {
    /// Retrieve a sparse vector by ID.
    ///
    /// Returns `None` if:
    /// - ID does not exist
    /// - ID has been deleted (soft delete)
    ///
    /// # Arguments
    /// * `id` - The SparseId of the vector to retrieve
    ///
    /// # Returns
    /// * `Some(SparseVector)` - The vector if it exists and is not deleted
    /// * `None` - If ID is invalid or deleted
    ///
    /// # Example
    /// ```rust
    /// let id = storage.insert(&sparse_vec)?;
    /// let retrieved = storage.get(id);
    /// assert!(retrieved.is_some());
    ///
    /// storage.delete(id)?;
    /// let after_delete = storage.get(id);
    /// assert!(after_delete.is_none());
    /// ```
    pub fn get(&self, id: SparseId) -> Option<SparseVector> {
        let index = id.0 as usize;

        // Check bounds
        if index >= self.offsets.len() {
            return None;
        }

        // Check deletion bitmap - EARLY EXIT if deleted
        if self.deleted[index] {
            return None;
        }

        // Calculate slice bounds
        let start = self.offsets[index] as usize;
        let end = if index + 1 < self.offsets.len() {
            self.offsets[index + 1] as usize
        } else {
            self.indices.len()
        };

        // Extract slices
        let indices = self.indices[start..end].to_vec();
        let values = self.values[start..end].to_vec();
        let dim = self.dims[index];

        // Reconstruct SparseVector (validation skipped - data already validated on insert)
        // Use unchecked constructor or trust internal invariant
        SparseVector::new_unchecked(indices, values, dim)
    }

    /// Internal constructor that skips validation.
    /// Only used when reconstructing from trusted storage.
    fn new_unchecked(indices: Vec<u32>, values: Vec<f32>, dim: u32) -> Option<SparseVector> {
        // Storage data is already validated, so we can construct directly
        Some(SparseVector {
            indices,
            values,
            dim,
        })
    }
}
```

**Acceptance Criteria:**
- [ ] `get()` returns `None` for deleted vectors
- [ ] `get()` returns `Some(vec)` for active vectors
- [ ] `get()` returns `None` for non-existent IDs
- [ ] Deletion check is performed before data extraction (early exit)
- [ ] Existing tests for `get()` still pass

**Estimated Duration:** 25 minutes

**Agent:** RUST_ENGINEER

---

### W38.5.4: Update `iter()` to Skip Deleted Vectors

**Objective:** Modify iterator to exclude deleted vectors.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

impl SparseStorage {
    /// Iterate over all active (non-deleted) sparse vectors.
    ///
    /// Returns an iterator of `(SparseId, SparseVector)` pairs,
    /// skipping any vectors that have been deleted.
    ///
    /// # Example
    /// ```rust
    /// // Insert 3 vectors, delete the middle one
    /// let id1 = storage.insert(&vec1)?;
    /// let id2 = storage.insert(&vec2)?;
    /// let id3 = storage.insert(&vec3)?;
    /// storage.delete(id2)?;
    ///
    /// // Iteration only yields id1 and id3
    /// let active: Vec<_> = storage.iter().collect();
    /// assert_eq!(active.len(), 2);
    /// assert_eq!(active[0].0, id1);
    /// assert_eq!(active[1].0, id3);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (SparseId, SparseVector)> + '_ {
        SparseStorageIter {
            storage: self,
            current_index: 0,
        }
    }

    /// Iterate over all active (non-deleted) sparse vector IDs.
    ///
    /// More efficient than `iter()` when you only need IDs.
    pub fn iter_ids(&self) -> impl Iterator<Item = SparseId> + '_ {
        (0..self.offsets.len())
            .filter(move |&idx| !self.deleted[idx])
            .map(|idx| SparseId(idx as u64))
    }
}

/// Iterator over active sparse vectors in storage.
pub struct SparseStorageIter<'a> {
    storage: &'a SparseStorage,
    current_index: usize,
}

impl<'a> Iterator for SparseStorageIter<'a> {
    type Item = (SparseId, SparseVector);

    fn next(&mut self) -> Option<Self::Item> {
        // Skip deleted entries
        while self.current_index < self.storage.offsets.len() {
            let idx = self.current_index;
            self.current_index += 1;

            // Skip if deleted
            if self.storage.deleted[idx] {
                continue;
            }

            // Get vector at this index
            let id = SparseId(idx as u64);
            if let Some(vec) = self.storage.get(id) {
                return Some((id, vec));
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Upper bound: remaining entries
        let remaining = self.storage.offsets.len().saturating_sub(self.current_index);
        (0, Some(remaining))
    }
}
```

**Acceptance Criteria:**
- [ ] `iter()` skips deleted vectors
- [ ] `iter()` yields `(SparseId, SparseVector)` pairs
- [ ] `iter_ids()` skips deleted vectors (ID-only iteration)
- [ ] Iterator is lazy (processes on demand)
- [ ] Empty storage returns empty iterator
- [ ] All-deleted storage returns empty iterator

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W38.5.5: Implement `deleted_count(&self) -> usize`

**Objective:** Track number of deleted vectors for monitoring and compaction decisions.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

impl SparseStorage {
    /// Get the count of deleted vectors.
    ///
    /// Useful for:
    /// - Monitoring storage fragmentation
    /// - Deciding when to run compaction
    /// - Debugging and testing
    ///
    /// # Returns
    /// Number of vectors marked as deleted.
    ///
    /// # Example
    /// ```rust
    /// let mut storage = SparseStorage::new();
    /// assert_eq!(storage.deleted_count(), 0);
    ///
    /// let id1 = storage.insert(&vec1)?;
    /// let id2 = storage.insert(&vec2)?;
    /// assert_eq!(storage.deleted_count(), 0);
    ///
    /// storage.delete(id1)?;
    /// assert_eq!(storage.deleted_count(), 1);
    ///
    /// storage.delete(id2)?;
    /// assert_eq!(storage.deleted_count(), 2);
    /// ```
    pub fn deleted_count(&self) -> usize {
        self.deleted.count_ones()
    }

    /// Get the count of active (non-deleted) vectors.
    ///
    /// This is the effective size of the storage.
    ///
    /// # Returns
    /// `total_count() - deleted_count()`
    pub fn active_count(&self) -> usize {
        self.offsets.len() - self.deleted_count()
    }

    /// Get the total count of vectors (including deleted).
    ///
    /// This represents the raw storage size.
    pub fn total_count(&self) -> usize {
        self.offsets.len()
    }

    /// Get the deletion ratio (deleted / total).
    ///
    /// Useful for deciding when to compact:
    /// - Ratio > 0.5: Consider compaction
    /// - Ratio > 0.7: Recommend compaction
    ///
    /// # Returns
    /// Ratio in [0.0, 1.0], or 0.0 if storage is empty.
    pub fn deletion_ratio(&self) -> f32 {
        if self.offsets.is_empty() {
            return 0.0;
        }
        self.deleted_count() as f32 / self.offsets.len() as f32
    }
}
```

**Acceptance Criteria:**
- [ ] `deleted_count()` returns correct count after deletions
- [ ] `active_count()` = `total_count()` - `deleted_count()`
- [ ] `total_count()` includes deleted vectors
- [ ] `deletion_ratio()` returns 0.0 for empty storage
- [ ] All counts are O(n) or better (BitVec count_ones is efficient)

**Estimated Duration:** 20 minutes

**Agent:** RUST_ENGINEER

---

### W38.5.6: Add Unit Tests for Deletion

**Objective:** Comprehensive unit tests for deletion functionality.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (in #[cfg(test)] module)

#[cfg(test)]
mod deletion_tests {
    use super::*;

    fn sample_sparse(id: u32) -> SparseVector {
        SparseVector::new(
            vec![0, id, id + 10],
            vec![0.1, 0.2, 0.3],
            100,
        ).unwrap()
    }

    #[test]
    fn test_delete_returns_true_first_time() {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&sample_sparse(1)).unwrap();

        let result = storage.delete(id);
        assert!(result.is_ok());
        assert!(result.unwrap(), "First delete should return true");
    }

    #[test]
    fn test_delete_returns_false_second_time() {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&sample_sparse(1)).unwrap();

        storage.delete(id).unwrap();
        let result = storage.delete(id);

        assert!(result.is_ok());
        assert!(!result.unwrap(), "Second delete should return false");
    }

    #[test]
    fn test_delete_nonexistent_id_fails() {
        let mut storage = SparseStorage::new();
        let fake_id = SparseId(999);

        let result = storage.delete(fake_id);
        assert!(matches!(result, Err(SparseError::IdNotFound(999))));
    }

    #[test]
    fn test_is_deleted_false_before_delete() {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&sample_sparse(1)).unwrap();

        assert!(!storage.is_deleted(id));
    }

    #[test]
    fn test_is_deleted_true_after_delete() {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&sample_sparse(1)).unwrap();

        storage.delete(id).unwrap();
        assert!(storage.is_deleted(id));
    }

    #[test]
    fn test_is_deleted_true_for_nonexistent() {
        let storage = SparseStorage::new();
        assert!(storage.is_deleted(SparseId(999)));
    }

    #[test]
    fn test_get_returns_none_after_delete() {
        let mut storage = SparseStorage::new();
        let vec = sample_sparse(1);
        let id = storage.insert(&vec).unwrap();

        assert!(storage.get(id).is_some(), "Before delete");

        storage.delete(id).unwrap();
        assert!(storage.get(id).is_none(), "After delete");
    }

    #[test]
    fn test_iter_skips_deleted() {
        let mut storage = SparseStorage::new();
        let id1 = storage.insert(&sample_sparse(1)).unwrap();
        let id2 = storage.insert(&sample_sparse(2)).unwrap();
        let id3 = storage.insert(&sample_sparse(3)).unwrap();

        // Delete middle one
        storage.delete(id2).unwrap();

        let ids: Vec<_> = storage.iter().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(!ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn test_iter_ids_skips_deleted() {
        let mut storage = SparseStorage::new();
        let id1 = storage.insert(&sample_sparse(1)).unwrap();
        let id2 = storage.insert(&sample_sparse(2)).unwrap();
        let id3 = storage.insert(&sample_sparse(3)).unwrap();

        storage.delete(id2).unwrap();

        let ids: Vec<_> = storage.iter_ids().collect();
        assert_eq!(ids, vec![id1, id3]);
    }

    #[test]
    fn test_deleted_count_increments() {
        let mut storage = SparseStorage::new();
        let id1 = storage.insert(&sample_sparse(1)).unwrap();
        let id2 = storage.insert(&sample_sparse(2)).unwrap();

        assert_eq!(storage.deleted_count(), 0);

        storage.delete(id1).unwrap();
        assert_eq!(storage.deleted_count(), 1);

        storage.delete(id2).unwrap();
        assert_eq!(storage.deleted_count(), 2);
    }

    #[test]
    fn test_active_count_decrements() {
        let mut storage = SparseStorage::new();
        let id1 = storage.insert(&sample_sparse(1)).unwrap();
        let _id2 = storage.insert(&sample_sparse(2)).unwrap();

        assert_eq!(storage.active_count(), 2);

        storage.delete(id1).unwrap();
        assert_eq!(storage.active_count(), 1);
    }

    #[test]
    fn test_total_count_unchanged_after_delete() {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&sample_sparse(1)).unwrap();

        assert_eq!(storage.total_count(), 1);

        storage.delete(id).unwrap();
        assert_eq!(storage.total_count(), 1); // Still 1 (soft delete)
    }

    #[test]
    fn test_deletion_ratio() {
        let mut storage = SparseStorage::new();

        // Empty storage
        assert_eq!(storage.deletion_ratio(), 0.0);

        // Insert 4 vectors
        for i in 0..4 {
            storage.insert(&sample_sparse(i)).unwrap();
        }
        assert_eq!(storage.deletion_ratio(), 0.0);

        // Delete 1 of 4 = 25%
        storage.delete(SparseId(0)).unwrap();
        assert!((storage.deletion_ratio() - 0.25).abs() < 0.01);

        // Delete 2 of 4 = 50%
        storage.delete(SparseId(1)).unwrap();
        assert!((storage.deletion_ratio() - 0.50).abs() < 0.01);
    }

    #[test]
    fn test_delete_all_then_iter() {
        let mut storage = SparseStorage::new();
        let id1 = storage.insert(&sample_sparse(1)).unwrap();
        let id2 = storage.insert(&sample_sparse(2)).unwrap();

        storage.delete(id1).unwrap();
        storage.delete(id2).unwrap();

        let count = storage.iter().count();
        assert_eq!(count, 0, "All deleted, iter should be empty");
    }

    #[test]
    fn test_exists_vs_is_deleted() {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&sample_sparse(1)).unwrap();
        let fake_id = SparseId(999);

        // Active vector: exists=true, is_deleted=false
        assert!(storage.exists(id));
        assert!(!storage.is_deleted(id));

        // Deleted vector: exists=true, is_deleted=true
        storage.delete(id).unwrap();
        assert!(storage.exists(id));
        assert!(storage.is_deleted(id));

        // Non-existent: exists=false, is_deleted=true
        assert!(!storage.exists(fake_id));
        assert!(storage.is_deleted(fake_id));
    }
}
```

**Acceptance Criteria:**
- [ ] Test: `delete` returns true on first call
- [ ] Test: `delete` returns false on second call
- [ ] Test: `delete` fails for non-existent IDs
- [ ] Test: `is_deleted` before and after delete
- [ ] Test: `get` returns None after delete
- [ ] Test: `iter` skips deleted
- [ ] Test: `iter_ids` skips deleted
- [ ] Test: `deleted_count` increments correctly
- [ ] Test: `active_count` decrements correctly
- [ ] Test: `total_count` unchanged after delete
- [ ] Test: `deletion_ratio` correct
- [ ] Test: all deleted -> empty iterator
- [ ] Test: `exists` vs `is_deleted` semantics
- [ ] All tests pass: `cargo test --features sparse deletion`

**Estimated Duration:** 45 minutes

**Agent:** TEST_ENGINEER

---

### W38.5.7: Add Property Tests for Delete/Get Invariants

**Objective:** Verify delete/get consistency with proptest.

**Rust Implementation:**

```rust
// tests/sparse_storage_deletion_test.rs

//! Property tests for SparseStorage deletion operations.

use proptest::prelude::*;
use edgevec::sparse::{SparseVector, SparseStorage, SparseId, SparseError};

/// Maximum vectors to insert in property tests
const MAX_VECTORS: usize = 100;

/// Strategy to generate a SparseVector
fn arb_sparse_vector() -> impl Strategy<Value = SparseVector> {
    (1u32..=1000, 1usize..=50).prop_flat_map(|(dim, nnz)| {
        let indices = proptest::collection::btree_set(0u32..dim, nnz)
            .prop_map(|set| set.into_iter().collect::<Vec<_>>());
        let values = proptest::collection::vec(
            prop::num::f32::NORMAL.prop_filter("nonzero", |v| *v != 0.0),
            nnz
        );
        (Just(dim), indices, values)
    })
    .prop_map(|(dim, indices, values)| {
        SparseVector::new(indices, values, dim).expect("valid vector")
    })
}

/// Strategy for a sequence of insert/delete operations
#[derive(Clone, Debug)]
enum Operation {
    Insert(SparseVector),
    Delete(usize),  // Index into inserted IDs
}

fn arb_operation(max_inserts: usize) -> impl Strategy<Value = Operation> {
    prop_oneof![
        3 => arb_sparse_vector().prop_map(Operation::Insert),
        1 => (0..max_inserts).prop_map(Operation::Delete),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Property: get(id) returns None after delete(id)
    #[test]
    fn prop_get_none_after_delete(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        // Before delete: get returns Some
        prop_assert!(storage.get(id).is_some());

        // After delete: get returns None
        storage.delete(id).unwrap();
        prop_assert!(storage.get(id).is_none());
    }

    /// Property: delete(id) twice -> second returns false
    #[test]
    fn prop_double_delete_returns_false(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        let first = storage.delete(id).unwrap();
        let second = storage.delete(id).unwrap();

        prop_assert!(first, "First delete should return true");
        prop_assert!(!second, "Second delete should return false");
    }

    /// Property: is_deleted(id) == true after delete(id)
    #[test]
    fn prop_is_deleted_after_delete(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        prop_assert!(!storage.is_deleted(id), "Before delete");
        storage.delete(id).unwrap();
        prop_assert!(storage.is_deleted(id), "After delete");
    }

    /// Property: active_count + deleted_count == total_count
    #[test]
    fn prop_count_invariant(
        vecs in proptest::collection::vec(arb_sparse_vector(), 1..=20),
        delete_indices in proptest::collection::vec(0usize..20, 0..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids = Vec::new();

        // Insert all vectors
        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        // Delete some (ignore out of bounds)
        for idx in delete_indices {
            if idx < ids.len() {
                let _ = storage.delete(ids[idx]);
            }
        }

        // Invariant check
        prop_assert_eq!(
            storage.active_count() + storage.deleted_count(),
            storage.total_count(),
            "active + deleted must equal total"
        );
    }

    /// Property: iter().count() == active_count()
    #[test]
    fn prop_iter_count_equals_active(
        vecs in proptest::collection::vec(arb_sparse_vector(), 1..=20),
        delete_indices in proptest::collection::vec(0usize..20, 0..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids = Vec::new();

        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        for idx in delete_indices {
            if idx < ids.len() {
                let _ = storage.delete(ids[idx]);
            }
        }

        let iter_count = storage.iter().count();
        let active = storage.active_count();

        prop_assert_eq!(iter_count, active,
            "iter().count() must equal active_count()");
    }

    /// Property: iter_ids() yields exactly the non-deleted IDs
    #[test]
    fn prop_iter_ids_matches_non_deleted(
        vecs in proptest::collection::vec(arb_sparse_vector(), 1..=20),
        delete_indices in proptest::collection::vec(0usize..20, 0..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids = Vec::new();
        let mut deleted_set = std::collections::HashSet::new();

        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        for idx in delete_indices {
            if idx < ids.len() {
                if storage.delete(ids[idx]).unwrap_or(false) {
                    deleted_set.insert(ids[idx]);
                }
            }
        }

        let iter_ids: std::collections::HashSet<_> = storage.iter_ids().collect();
        let expected: std::collections::HashSet<_> = ids.iter()
            .copied()
            .filter(|id| !deleted_set.contains(id))
            .collect();

        prop_assert_eq!(iter_ids, expected);
    }

    /// Property: delete(nonexistent) returns IdNotFound
    #[test]
    fn prop_delete_nonexistent_fails(
        vecs in proptest::collection::vec(arb_sparse_vector(), 0..=5)
    ) {
        let mut storage = SparseStorage::new();

        for vec in &vecs {
            storage.insert(vec).unwrap();
        }

        // ID beyond total count should fail
        let fake_id = SparseId(storage.total_count() as u64 + 100);
        let result = storage.delete(fake_id);

        prop_assert!(matches!(result, Err(SparseError::IdNotFound(_))));
    }

    /// Property: deletion_ratio in [0.0, 1.0]
    #[test]
    fn prop_deletion_ratio_bounded(
        vecs in proptest::collection::vec(arb_sparse_vector(), 1..=20),
        delete_indices in proptest::collection::vec(0usize..20, 0..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids = Vec::new();

        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        for idx in delete_indices {
            if idx < ids.len() {
                let _ = storage.delete(ids[idx]);
            }
        }

        let ratio = storage.deletion_ratio();
        prop_assert!(ratio >= 0.0 && ratio <= 1.0,
            "deletion_ratio must be in [0.0, 1.0], got {}", ratio);
    }
}
```

**Acceptance Criteria:**
- [ ] Property: `get(id) == None` after `delete(id)`
- [ ] Property: double delete returns `(true, false)`
- [ ] Property: `is_deleted(id) == true` after delete
- [ ] Property: `active_count + deleted_count == total_count`
- [ ] Property: `iter().count() == active_count()`
- [ ] Property: `iter_ids()` yields exactly non-deleted IDs
- [ ] Property: delete nonexistent returns `IdNotFound`
- [ ] Property: `deletion_ratio` in [0.0, 1.0]
- [ ] 500+ test cases per property
- [ ] All property tests pass: `cargo test --features sparse sparse_storage_deletion`

**Estimated Duration:** 45 minutes

**Agent:** TEST_ENGINEER

---

## Day 5 Checklist

- [ ] W38.5.1: `delete(&mut self, id)` implemented
- [ ] W38.5.2: `is_deleted(&self, id)` implemented
- [ ] W38.5.3: `get()` returns None for deleted vectors
- [ ] W38.5.4: `iter()` and `iter_ids()` skip deleted vectors
- [ ] W38.5.5: `deleted_count()`, `active_count()`, `total_count()`, `deletion_ratio()` implemented
- [ ] W38.5.6: Unit tests for all deletion methods (13+ tests)
- [ ] W38.5.7: Property tests (8+ properties, 500+ cases each)
- [ ] All tests pass
- [ ] No clippy warnings

---

## Day 5 Exit Criteria

| Criterion | Verification Command |
|:----------|:---------------------|
| All unit tests pass | `cargo test --features sparse deletion` |
| All property tests pass | `cargo test --features sparse sparse_storage_deletion` |
| 500+ cases per property | ProptestConfig in test file |
| No clippy warnings | `cargo clippy --features sparse -- -D warnings` |
| Documentation complete | `cargo doc --features sparse --no-deps` |

---

## Day 5 Verification Commands

```bash
# Run all deletion unit tests
cargo test --features sparse deletion -- --nocapture

# Run property tests
cargo test --features sparse sparse_storage_deletion

# Run with more cases (optional stress test)
PROPTEST_CASES=2000 cargo test --features sparse sparse_storage_deletion

# Check for warnings
cargo clippy --features sparse -- -D warnings

# Verify documentation builds
cargo doc --features sparse --no-deps --open
```

---

## Implementation Notes

### BitVec Usage

The `bitvec` crate provides efficient bit manipulation:

```rust
use bitvec::prelude::*;

// In SparseStorage::new()
deleted: bitvec![0; 0],  // Empty initially

// When inserting a new vector
self.deleted.push(false);

// When deleting
self.deleted.set(index, true);

// Count ones efficiently
self.deleted.count_ones()
```

### Soft Delete Only (No Compaction)

This phase implements **soft delete only**. Compaction (reclaiming space) is deferred to a future phase because:

1. Adds complexity (ID remapping, WAL entries)
2. Not needed for MVP functionality
3. Can be triggered manually or on save/load

### ID Stability

After soft delete:
- ID remains valid (exists returns true)
- ID cannot be reused until compaction
- Sequential ID assignment continues from next_id

---

## Day 5 Handoff

After completing Day 5:

**Artifacts Generated:**
- Updated `src/sparse/storage.rs` with deletion support
- `tests/sparse_storage_deletion_test.rs` with property tests

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 6 - SparseStorage Serialization (save/load)

---

*Agent: PLANNER*
*Status: [PROPOSED]*
*Date: 2026-01-09*

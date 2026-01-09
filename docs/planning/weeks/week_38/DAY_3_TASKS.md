# Week 38 Day 3: SparseStorage Get Operations and Iteration

**Date:** 2026-01-21
**Focus:** Implement `get()` method and iterator for SparseStorage
**Estimated Duration:** 3 hours
**Phase:** RFC-007 Implementation Phase 2 (Storage)
**Dependencies:** Day 2 (SparseStorage Insert) -- MUST BE COMPLETE

---

## Tasks

### W38.3.1: Implement `get(&self, id: SparseId) -> Option<SparseVector>`

**Objective:** Retrieve a sparse vector by ID, reconstructing from packed arrays.

**Rust Implementation:**

```rust
// src/sparse/storage.rs

impl SparseStorage {
    /// Retrieve a sparse vector by ID.
    ///
    /// Returns `None` if:
    /// - ID is out of bounds
    /// - Vector has been deleted
    ///
    /// # Complexity
    ///
    /// - Time: O(nnz) for cloning indices and values
    /// - Space: O(nnz) for the returned SparseVector
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100)?;
    /// let id = storage.insert(&vec)?;
    ///
    /// let retrieved = storage.get(id);
    /// assert!(retrieved.is_some());
    /// assert_eq!(retrieved.unwrap(), vec);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn get(&self, id: SparseId) -> Option<SparseVector> {
        let idx = id.0 as usize;

        // Bounds check
        if idx >= self.offsets.len().saturating_sub(1) {
            return None;
        }

        // Deletion check
        if self.deleted.get(idx).map_or(false, |bit| *bit) {
            return None;
        }

        // Get the range for this vector's data
        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;
        let dim = self.dims[idx];

        // Clone the indices and values from packed arrays
        let indices = self.indices[start..end].to_vec();
        let values = self.values[start..end].to_vec();

        // Safety: Data was validated on insert, so this should not fail.
        // We use unwrap_or_else to handle the theoretical case where
        // storage corruption occurred.
        SparseVector::new(indices, values, dim).ok()
    }
}
```

**Acceptance Criteria:**
- [ ] Returns `Some(SparseVector)` for valid, non-deleted IDs
- [ ] Returns `None` for out-of-bounds IDs
- [ ] Returns `None` for deleted IDs
- [ ] Reconstructed vector equals original (by value)
- [ ] O(nnz) time complexity
- [ ] Doc comments with example

**Test Cases:**

```rust
#[cfg(test)]
mod get_tests {
    use super::*;

    #[test]
    fn test_get_valid() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        let retrieved = storage.get(id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), vec);
    }

    #[test]
    fn test_get_out_of_bounds() {
        let storage = SparseStorage::new();
        let result = storage.get(SparseId(999));
        assert!(result.is_none());
    }

    #[test]
    fn test_get_deleted() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5], vec![1.0, 2.0], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        storage.delete(id).unwrap();
        assert!(storage.get(id).is_none());
    }

    #[test]
    fn test_get_multiple_vectors() {
        let mut storage = SparseStorage::new();

        let vec1 = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let vec2 = SparseVector::new(vec![5, 10], vec![2.0, 3.0], 100).unwrap();
        let vec3 = SparseVector::new(vec![0, 1, 2], vec![0.1, 0.2, 0.3], 100).unwrap();

        let id1 = storage.insert(&vec1).unwrap();
        let id2 = storage.insert(&vec2).unwrap();
        let id3 = storage.insert(&vec3).unwrap();

        assert_eq!(storage.get(id1), Some(vec1));
        assert_eq!(storage.get(id2), Some(vec2));
        assert_eq!(storage.get(id3), Some(vec3));
    }

    #[test]
    fn test_get_empty_storage() {
        let storage = SparseStorage::new();
        assert!(storage.get(SparseId(0)).is_none());
    }
}
```

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W38.3.2: Implement `get_indices(&self, id: SparseId) -> Option<&[u32]>` (Zero-Copy)

**Objective:** Provide zero-copy access to indices slice for performance-critical code paths.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

impl SparseStorage {
    /// Get indices slice for a sparse vector (zero-copy).
    ///
    /// Returns `None` if:
    /// - ID is out of bounds
    /// - Vector has been deleted
    ///
    /// # Use Case
    ///
    /// For performance-critical operations where you need to iterate
    /// indices without allocating. Useful for inverted index construction.
    ///
    /// # Complexity
    ///
    /// - Time: O(1)
    /// - Space: O(1) (no allocation)
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100)?;
    /// let id = storage.insert(&vec)?;
    ///
    /// let indices = storage.get_indices(id);
    /// assert_eq!(indices, Some(&[0u32, 5, 10][..]));
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn get_indices(&self, id: SparseId) -> Option<&[u32]> {
        let idx = id.0 as usize;

        // Bounds check
        if idx >= self.offsets.len().saturating_sub(1) {
            return None;
        }

        // Deletion check
        if self.deleted.get(idx).map_or(false, |bit| *bit) {
            return None;
        }

        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;

        Some(&self.indices[start..end])
    }
}
```

**Acceptance Criteria:**
- [ ] Returns `Some(&[u32])` for valid, non-deleted IDs
- [ ] Returns `None` for out-of-bounds IDs
- [ ] Returns `None` for deleted IDs
- [ ] O(1) time complexity (no cloning)
- [ ] Zero heap allocation
- [ ] Returned slice matches original vector's indices

**Test Cases:**

```rust
#[cfg(test)]
mod get_indices_tests {
    use super::*;

    #[test]
    fn test_get_indices_valid() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        let indices = storage.get_indices(id);
        assert_eq!(indices, Some(&[0u32, 5, 10][..]));
    }

    #[test]
    fn test_get_indices_out_of_bounds() {
        let storage = SparseStorage::new();
        assert!(storage.get_indices(SparseId(0)).is_none());
    }

    #[test]
    fn test_get_indices_deleted() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5], vec![1.0, 2.0], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        storage.delete(id).unwrap();
        assert!(storage.get_indices(id).is_none());
    }

    #[test]
    fn test_get_indices_multiple() {
        let mut storage = SparseStorage::new();

        let id1 = storage.insert(&SparseVector::new(vec![0], vec![1.0], 100).unwrap()).unwrap();
        let id2 = storage.insert(&SparseVector::new(vec![5, 10], vec![2.0, 3.0], 100).unwrap()).unwrap();

        assert_eq!(storage.get_indices(id1), Some(&[0u32][..]));
        assert_eq!(storage.get_indices(id2), Some(&[5u32, 10][..]));
    }
}
```

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W38.3.3: Implement `get_values(&self, id: SparseId) -> Option<&[f32]>` (Zero-Copy)

**Objective:** Provide zero-copy access to values slice for performance-critical code paths.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

impl SparseStorage {
    /// Get values slice for a sparse vector (zero-copy).
    ///
    /// Returns `None` if:
    /// - ID is out of bounds
    /// - Vector has been deleted
    ///
    /// # Use Case
    ///
    /// For performance-critical operations where you need to iterate
    /// values without allocating. Useful for dot product computation.
    ///
    /// # Complexity
    ///
    /// - Time: O(1)
    /// - Space: O(1) (no allocation)
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100)?;
    /// let id = storage.insert(&vec)?;
    ///
    /// let values = storage.get_values(id);
    /// assert!(values.is_some());
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn get_values(&self, id: SparseId) -> Option<&[f32]> {
        let idx = id.0 as usize;

        // Bounds check
        if idx >= self.offsets.len().saturating_sub(1) {
            return None;
        }

        // Deletion check
        if self.deleted.get(idx).map_or(false, |bit| *bit) {
            return None;
        }

        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;

        Some(&self.values[start..end])
    }
}
```

**Acceptance Criteria:**
- [ ] Returns `Some(&[f32])` for valid, non-deleted IDs
- [ ] Returns `None` for out-of-bounds IDs
- [ ] Returns `None` for deleted IDs
- [ ] O(1) time complexity (no cloning)
- [ ] Zero heap allocation
- [ ] Returned slice matches original vector's values

**Test Cases:**

```rust
#[cfg(test)]
mod get_values_tests {
    use super::*;

    #[test]
    fn test_get_values_valid() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        let values = storage.get_values(id);
        assert!(values.is_some());

        let values = values.unwrap();
        assert!((values[0] - 0.1).abs() < 1e-6);
        assert!((values[1] - 0.2).abs() < 1e-6);
        assert!((values[2] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_get_values_out_of_bounds() {
        let storage = SparseStorage::new();
        assert!(storage.get_values(SparseId(0)).is_none());
    }

    #[test]
    fn test_get_values_deleted() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5], vec![1.0, 2.0], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        storage.delete(id).unwrap();
        assert!(storage.get_values(id).is_none());
    }

    #[test]
    fn test_get_indices_and_values_consistent() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        let indices = storage.get_indices(id).unwrap();
        let values = storage.get_values(id).unwrap();

        assert_eq!(indices.len(), values.len());
        assert_eq!(indices.len(), 3);
    }
}
```

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W38.3.4: Implement `iter()` Method Returning Iterator

**Objective:** Implement iterator over live (non-deleted) sparse vectors in storage.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

/// Iterator over live sparse vectors in storage.
///
/// Yields `(SparseId, SparseVector)` pairs for all non-deleted vectors.
/// Deleted entries are skipped automatically.
pub struct SparseStorageIter<'a> {
    storage: &'a SparseStorage,
    current: usize,
}

impl<'a> Iterator for SparseStorageIter<'a> {
    type Item = (SparseId, SparseVector);

    fn next(&mut self) -> Option<Self::Item> {
        // Skip deleted entries
        while self.current < self.storage.len() {
            let id = SparseId(self.current as u64);

            // Check if this entry is deleted
            if self.storage.deleted.get(self.current).map_or(false, |bit| *bit) {
                self.current += 1;
                continue;
            }

            // Get the vector (will succeed since we checked deletion)
            if let Some(vec) = self.storage.get(id) {
                self.current += 1;
                return Some((id, vec));
            }

            self.current += 1;
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Lower bound: 0 (all remaining could be deleted)
        // Upper bound: remaining entries
        let remaining = self.storage.len().saturating_sub(self.current);
        (0, Some(remaining))
    }
}

impl SparseStorage {
    /// Iterate over all live (non-deleted) sparse vectors.
    ///
    /// # Complexity
    ///
    /// - Time: O(n * avg_nnz) total, where n is total vectors
    /// - Space: O(max_nnz) per yielded vector
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// storage.insert(&SparseVector::new(vec![0], vec![1.0], 100)?)?;
    /// storage.insert(&SparseVector::new(vec![5], vec![2.0], 100)?)?;
    ///
    /// for (id, vec) in storage.iter() {
    ///     println!("Vector {:?} has {} non-zero elements", id, vec.nnz());
    /// }
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn iter(&self) -> SparseStorageIter<'_> {
        SparseStorageIter {
            storage: self,
            current: 0,
        }
    }
}
```

**Acceptance Criteria:**
- [ ] Iterator yields `(SparseId, SparseVector)` pairs
- [ ] Deleted entries are skipped automatically
- [ ] Iteration order is ascending by ID
- [ ] Empty storage yields no elements
- [ ] All live vectors are visited exactly once
- [ ] `size_hint` provides correct upper bound

**Test Cases:**

```rust
#[cfg(test)]
mod iter_tests {
    use super::*;

    #[test]
    fn test_iter_empty() {
        let storage = SparseStorage::new();
        let count = storage.iter().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_iter_all_live() {
        let mut storage = SparseStorage::new();

        let v1 = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let v2 = SparseVector::new(vec![5], vec![2.0], 100).unwrap();
        let v3 = SparseVector::new(vec![10], vec![3.0], 100).unwrap();

        storage.insert(&v1).unwrap();
        storage.insert(&v2).unwrap();
        storage.insert(&v3).unwrap();

        let collected: Vec<_> = storage.iter().collect();
        assert_eq!(collected.len(), 3);

        assert_eq!(collected[0].0, SparseId(0));
        assert_eq!(collected[1].0, SparseId(1));
        assert_eq!(collected[2].0, SparseId(2));

        assert_eq!(collected[0].1, v1);
        assert_eq!(collected[1].1, v2);
        assert_eq!(collected[2].1, v3);
    }

    #[test]
    fn test_iter_skips_deleted() {
        let mut storage = SparseStorage::new();

        let v1 = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let v2 = SparseVector::new(vec![5], vec![2.0], 100).unwrap();
        let v3 = SparseVector::new(vec![10], vec![3.0], 100).unwrap();

        let id1 = storage.insert(&v1).unwrap();
        let _id2 = storage.insert(&v2).unwrap();
        let id3 = storage.insert(&v3).unwrap();

        // Delete middle vector
        storage.delete(SparseId(1)).unwrap();

        let collected: Vec<_> = storage.iter().collect();
        assert_eq!(collected.len(), 2);

        assert_eq!(collected[0].0, id1);
        assert_eq!(collected[1].0, id3);
    }

    #[test]
    fn test_iter_all_deleted() {
        let mut storage = SparseStorage::new();

        let vec = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let id = storage.insert(&vec).unwrap();
        storage.delete(id).unwrap();

        let count = storage.iter().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_iter_size_hint() {
        let mut storage = SparseStorage::new();

        let vec = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        storage.insert(&vec).unwrap();
        storage.insert(&vec).unwrap();
        storage.insert(&vec).unwrap();

        let iter = storage.iter();
        let (lower, upper) = iter.size_hint();

        assert_eq!(lower, 0);
        assert_eq!(upper, Some(3));
    }

    #[test]
    fn test_iter_verifies_data_integrity() {
        let mut storage = SparseStorage::new();

        let v1 = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let v2 = SparseVector::new(vec![1, 2], vec![1.0, 2.0], 50).unwrap();

        storage.insert(&v1).unwrap();
        storage.insert(&v2).unwrap();

        for (id, vec) in storage.iter() {
            match id.0 {
                0 => {
                    assert_eq!(vec.nnz(), 3);
                    assert_eq!(vec.dim(), 100);
                }
                1 => {
                    assert_eq!(vec.nnz(), 2);
                    assert_eq!(vec.dim(), 50);
                }
                _ => panic!("Unexpected ID"),
            }
        }
    }
}
```

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W38.3.5: Add Unit Tests for Get Operations

**Objective:** Comprehensive unit tests for edge cases and integration scenarios.

**Rust Implementation:**

```rust
// tests/sparse_storage_get_test.rs

use edgevec::sparse::{SparseStorage, SparseVector, SparseId, SparseError};

#[test]
fn test_roundtrip_single_vector() {
    let mut storage = SparseStorage::new();
    let original = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();

    let id = storage.insert(&original).unwrap();
    let retrieved = storage.get(id).unwrap();

    assert_eq!(original, retrieved);
}

#[test]
fn test_roundtrip_many_vectors() {
    let mut storage = SparseStorage::new();
    let mut originals = Vec::new();
    let mut ids = Vec::new();

    // Insert 100 vectors with varying nnz
    for i in 0..100 {
        let nnz = (i % 10) + 1;
        let indices: Vec<u32> = (0..nnz as u32).map(|j| j * 10 + i as u32).collect();
        let values: Vec<f32> = (0..nnz).map(|j| (j + 1) as f32 * 0.1).collect();
        let dim = 1000 + i as u32;

        let vec = SparseVector::new(indices, values, dim).unwrap();
        let id = storage.insert(&vec).unwrap();

        originals.push(vec);
        ids.push(id);
    }

    // Verify all
    for (original, id) in originals.iter().zip(ids.iter()) {
        let retrieved = storage.get(*id).unwrap();
        assert_eq!(original, &retrieved);
    }
}

#[test]
fn test_zero_copy_access_matches_get() {
    let mut storage = SparseStorage::new();
    let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
    let id = storage.insert(&vec).unwrap();

    let full_vec = storage.get(id).unwrap();
    let indices = storage.get_indices(id).unwrap();
    let values = storage.get_values(id).unwrap();

    assert_eq!(full_vec.indices(), indices);
    assert_eq!(full_vec.values(), values);
}

#[test]
fn test_interleaved_insert_delete_get() {
    let mut storage = SparseStorage::new();

    // Insert 3
    let v1 = SparseVector::singleton(0, 1.0, 100).unwrap();
    let v2 = SparseVector::singleton(1, 2.0, 100).unwrap();
    let v3 = SparseVector::singleton(2, 3.0, 100).unwrap();

    let id1 = storage.insert(&v1).unwrap();
    let id2 = storage.insert(&v2).unwrap();
    let id3 = storage.insert(&v3).unwrap();

    // Delete middle
    storage.delete(id2).unwrap();

    // Verify
    assert!(storage.get(id1).is_some());
    assert!(storage.get(id2).is_none());
    assert!(storage.get(id3).is_some());

    // Insert new
    let v4 = SparseVector::singleton(4, 4.0, 100).unwrap();
    let id4 = storage.insert(&v4).unwrap();

    // Verify all
    assert_eq!(storage.get(id1), Some(v1));
    assert!(storage.get(id2).is_none());
    assert_eq!(storage.get(id3), Some(v3));
    assert_eq!(storage.get(id4), Some(v4));
}

#[test]
fn test_large_nnz_vector() {
    let mut storage = SparseStorage::new();

    // Vector with 1000 non-zero elements
    let indices: Vec<u32> = (0..1000).collect();
    let values: Vec<f32> = (0..1000).map(|i| i as f32 * 0.001).collect();
    let vec = SparseVector::new(indices, values, 10000).unwrap();

    let id = storage.insert(&vec).unwrap();
    let retrieved = storage.get(id).unwrap();

    assert_eq!(vec, retrieved);
    assert_eq!(retrieved.nnz(), 1000);
}

#[test]
fn test_iter_live_count_consistency() {
    let mut storage = SparseStorage::new();

    let vec = SparseVector::singleton(0, 1.0, 100).unwrap();

    // Insert 10
    for _ in 0..10 {
        storage.insert(&vec).unwrap();
    }

    // Delete 3
    storage.delete(SparseId(2)).unwrap();
    storage.delete(SparseId(5)).unwrap();
    storage.delete(SparseId(8)).unwrap();

    let iter_count = storage.iter().count();
    let live_count = storage.live_count();

    assert_eq!(iter_count, live_count);
    assert_eq!(iter_count, 7);
}
```

**Acceptance Criteria:**
- [ ] Roundtrip test for single vector
- [ ] Roundtrip test for many vectors
- [ ] Zero-copy access matches full get
- [ ] Interleaved insert/delete/get works correctly
- [ ] Large nnz vectors work correctly
- [ ] Iterator count matches live_count

**Estimated Duration:** 30 minutes

**Agent:** TEST_ENGINEER

---

## Day 3 Checklist

- [ ] W38.3.1: `get()` method implemented
- [ ] W38.3.2: `get_indices()` zero-copy method implemented
- [ ] W38.3.3: `get_values()` zero-copy method implemented
- [ ] W38.3.4: `iter()` method with `SparseStorageIter` implemented
- [ ] W38.3.5: Unit tests for all get operations
- [ ] All unit tests pass
- [ ] `cargo clippy --features sparse -- -D warnings` passes
- [ ] `cargo test --features sparse` passes

---

## Verification Commands

```bash
# Run sparse storage tests
cargo test --features sparse sparse_storage -- --nocapture

# Run get-specific tests
cargo test --features sparse get_ -- --nocapture

# Run iterator tests
cargo test --features sparse iter_ -- --nocapture

# Run integration tests
cargo test --features sparse --test sparse_storage_get_test

# Clippy check
cargo clippy --features sparse -- -D warnings

# Full test suite
cargo test --features sparse
```

---

## Day 3 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `get()` returns correct `SparseVector` | `test_get_valid`, `test_roundtrip_*` |
| `get()` returns `None` for deleted | `test_get_deleted` |
| `get_indices()` is zero-copy | `test_get_indices_valid` |
| `get_values()` is zero-copy | `test_get_values_valid` |
| `iter()` skips deleted entries | `test_iter_skips_deleted` |
| All methods documented | `cargo doc --features sparse` |
| Clippy clean | `cargo clippy --features sparse -- -D warnings` |

---

## Day 3 Handoff

After completing Day 3:

**Artifacts Generated:**
- `src/sparse/storage.rs` updated with get/iter methods
- `tests/sparse_storage_get_test.rs` integration tests
- Unit tests in `src/sparse/storage.rs`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 4 -- SparseStorage Delete and Compaction

---

*Agent: PLANNER + RUST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2026-01-09*

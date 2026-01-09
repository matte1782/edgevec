# Week 38: Sparse Vectors Phase 2 — SparseStorage Implementation

**Date Range:** 2026-01-19 to 2026-01-24
**Focus:** Implement SparseStorage with packed arrays, serialization, and deletion
**Hours:** 16h (2-3h/day)
**Status:** [ ] PROPOSED
**Depends On:** Week 37 COMPLETE (SparseVector, SparseError, metrics), RFC-007 (APPROVED)

---

## Context

Week 38 begins Phase 2 of sparse vector implementation as defined in RFC-007. This week focuses on the storage layer that enables persistence and retrieval of sparse vectors.

**RFC-007 Phase 2 Goals:**
- Implement `SparseStorage` with packed arrays
- Add serialization (same pattern as dense storage)
- Add deletion support (BitVec)
- WAL integration (entry type 2 = Sparse Insert)

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

**Performance Targets (from RFC-007):**
- Insert sparse vector: P50 <50us, P99 <100us
- Storage overhead: 8 bytes per non-zero element + 12 bytes per vector

**Memory Estimate (100k sparse vectors, avg 50 non-zero):**
```
indices: 100k * 50 * 4 bytes = 20 MB
values:  100k * 50 * 4 bytes = 20 MB
offsets: 100k * 4 bytes      = 0.4 MB
dims:    100k * 4 bytes      = 0.4 MB
deleted: 100k / 8 bytes      = 12.5 KB
Total: ~41 MB
```

---

## Week 38 Tasks Overview

| Day | Date | Task | Hours | Priority |
|:----|:-----|:-----|:------|:---------|
| 1 | 2026-01-19 | SparseStorage struct definition + SparseId | 2h | P0 |
| 2 | 2026-01-20 | Insert operation | 3h | P0 |
| 3 | 2026-01-21 | Get operation + iterator | 3h | P0 |
| 4 | 2026-01-22 | Serialization (save/load) | 3h | P0 |
| 5 | 2026-01-23 | Deletion support | 3h | P0 |
| 6 | 2026-01-24 | Benchmarks + Hostile Review | 2h | P0 |

**Total:** 16 hours

---

## Day 1: SparseStorage Struct Definition (2h)

**Date:** 2026-01-19
**Goal:** Define SparseStorage struct and SparseId type

### Tasks

#### W38.1.1: Define `SparseId` type (30min)

**Objective:** Create unique identifier for sparse vectors.

**File:** `src/sparse/storage.rs`

```rust
/// Unique identifier for sparse vectors.
///
/// Uses u64 for compatibility with dense VectorId.
/// High bit reserved for future hybrid ID scheme.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SparseId(pub u64);

impl SparseId {
    /// Create a new SparseId from a u64.
    #[inline]
    pub fn new(id: u64) -> Self {
        SparseId(id)
    }

    /// Get the underlying u64 value.
    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for SparseId {
    fn from(id: u64) -> Self {
        SparseId(id)
    }
}

impl From<SparseId> for u64 {
    fn from(id: SparseId) -> Self {
        id.0
    }
}
```

**Acceptance Criteria:**
- [ ] `SparseId` is Copy, Clone, Debug, PartialEq, Eq, Hash
- [ ] Derives Serialize, Deserialize for persistence
- [ ] Conversion traits to/from u64
- [ ] Documented with purpose and usage

---

#### W38.1.2: Define `SparseStorage` struct (45min)

**Objective:** Create the packed storage structure.

**File:** `src/sparse/storage.rs`

```rust
use bitvec::prelude::*;
use crate::sparse::{SparseVector, SparseError};

/// Packed storage for multiple sparse vectors.
///
/// # Memory Layout
///
/// All vectors' indices and values are concatenated into packed arrays.
/// The `offsets` array tracks where each vector starts:
///
/// ```text
/// Vector 0: indices[0..offsets[1]], values[0..offsets[1]]
/// Vector 1: indices[offsets[1]..offsets[2]], values[offsets[1]..offsets[2]]
/// ...
/// ```
///
/// # Deletion
///
/// Soft deletion via BitVec. Deleted slots are marked but not reclaimed
/// until compaction (future optimization).
///
/// # Thread Safety
///
/// NOT thread-safe. Wrap in Arc<RwLock<>> for concurrent access.
#[derive(Debug)]
pub struct SparseStorage {
    /// Packed indices: all vectors' indices concatenated
    indices: Vec<u32>,
    /// Packed values: all vectors' values concatenated
    values: Vec<f32>,
    /// Offsets into packed arrays: offsets[i] = start of vector i
    /// offsets[len] = total elements (sentinel)
    offsets: Vec<u32>,
    /// Maximum dimension for each stored vector
    dims: Vec<u32>,
    /// Deletion bitmap: true = deleted
    deleted: BitVec,
    /// Next ID to assign (monotonically increasing)
    next_id: u64,
}
```

**Acceptance Criteria:**
- [ ] All fields from RFC-007 design included
- [ ] Doc comments explain memory layout
- [ ] Derive Debug for diagnostics
- [ ] Deletion strategy documented

---

#### W38.1.3: Implement `new()` and capacity estimation (30min)

**Objective:** Constructors with pre-allocation support.

```rust
impl SparseStorage {
    /// Create empty storage.
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            values: Vec::new(),
            offsets: vec![0], // Sentinel for first vector
            dims: Vec::new(),
            deleted: BitVec::new(),
            next_id: 0,
        }
    }

    /// Create with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `num_vectors` - Expected number of vectors
    /// * `avg_nnz` - Average non-zeros per vector
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseStorage;
    ///
    /// // Pre-allocate for 10k vectors with ~50 non-zeros each
    /// let storage = SparseStorage::with_capacity(10_000, 50);
    /// ```
    pub fn with_capacity(num_vectors: usize, avg_nnz: usize) -> Self {
        let total_elements = num_vectors * avg_nnz;
        Self {
            indices: Vec::with_capacity(total_elements),
            values: Vec::with_capacity(total_elements),
            offsets: {
                let mut v = Vec::with_capacity(num_vectors + 1);
                v.push(0);
                v
            },
            dims: Vec::with_capacity(num_vectors),
            deleted: BitVec::with_capacity(num_vectors),
            next_id: 0,
        }
    }

    /// Returns the number of vectors (including deleted).
    #[inline]
    pub fn len(&self) -> usize {
        self.dims.len()
    }

    /// Returns true if no vectors are stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dims.is_empty()
    }

    /// Returns the number of non-deleted vectors.
    pub fn live_count(&self) -> usize {
        self.deleted.count_zeros()
    }

    /// Returns total memory used in bytes (approximate).
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of_val(&self.indices[..])
            + std::mem::size_of_val(&self.values[..])
            + std::mem::size_of_val(&self.offsets[..])
            + std::mem::size_of_val(&self.dims[..])
            + (self.deleted.len() + 7) / 8
    }
}

impl Default for SparseStorage {
    fn default() -> Self {
        Self::new()
    }
}
```

**Acceptance Criteria:**
- [ ] `new()` creates empty storage with offset sentinel
- [ ] `with_capacity()` pre-allocates for expected load
- [ ] `len()` returns total vectors (including deleted)
- [ ] `live_count()` returns non-deleted count
- [ ] `memory_usage()` returns approximate bytes
- [ ] Implements Default

---

#### W38.1.4: Update module exports (15min)

**File:** `src/sparse/mod.rs`

```rust
mod storage;

pub use storage::{SparseStorage, SparseId};
```

**Acceptance Criteria:**
- [ ] `SparseStorage` exported from sparse module
- [ ] `SparseId` exported from sparse module
- [ ] `cargo check --features sparse` passes

---

### Day 1 Checklist

- [ ] W38.1.1: `SparseId` type defined
- [ ] W38.1.2: `SparseStorage` struct defined
- [ ] W38.1.3: `new()`, `with_capacity()`, accessors implemented
- [ ] W38.1.4: Module exports updated
- [ ] `cargo check --features sparse` passes
- [ ] `cargo clippy --features sparse -- -D warnings` passes

### Day 1 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `SparseStorage::new()` compiles | `cargo check` |
| `SparseId` is Copy + Hash | Derive check |
| Memory layout matches RFC-007 | Code review |
| All doc comments present | `cargo doc` |

---

## Day 2: Insert Operation (3h)

**Date:** 2026-01-20
**Goal:** Implement insert with validation and ID assignment

### Tasks

#### W38.2.1: Implement `insert()` method (1h 30min)

**Objective:** Add sparse vector to packed storage.

```rust
impl SparseStorage {
    /// Insert a sparse vector into storage.
    ///
    /// # Arguments
    ///
    /// * `vector` - Validated SparseVector to insert
    ///
    /// # Returns
    ///
    /// `SparseId` assigned to the inserted vector.
    ///
    /// # Complexity
    ///
    /// - Time: O(nnz) - copying indices and values
    /// - Space: O(nnz) - new storage for the vector
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let vector = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100)?;
    /// let id = storage.insert(&vector)?;
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn insert(&mut self, vector: &SparseVector) -> Result<SparseId, SparseError> {
        // Assign ID
        let id = SparseId::new(self.next_id);
        self.next_id += 1;

        // Append indices
        self.indices.extend_from_slice(vector.indices());

        // Append values
        self.values.extend_from_slice(vector.values());

        // Record new offset (end of this vector = start of next)
        let new_offset = self.indices.len() as u32;
        self.offsets.push(new_offset);

        // Record dimension
        self.dims.push(vector.dim());

        // Mark as not deleted
        self.deleted.push(false);

        Ok(id)
    }
}
```

**Acceptance Criteria:**
- [ ] IDs are monotonically increasing
- [ ] Indices and values appended to packed arrays
- [ ] Offset sentinel updated
- [ ] Deletion bit set to false
- [ ] Returns assigned SparseId

---

#### W38.2.2: Implement `insert_batch()` method (45min)

**Objective:** Efficient bulk insertion.

```rust
impl SparseStorage {
    /// Insert multiple sparse vectors in a batch.
    ///
    /// More efficient than repeated single inserts due to
    /// reduced reallocation.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Slice of validated SparseVectors
    ///
    /// # Returns
    ///
    /// Vector of assigned SparseIds in order.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let vectors = vec![
    ///     SparseVector::singleton(0, 1.0, 100)?,
    ///     SparseVector::singleton(1, 2.0, 100)?,
    /// ];
    /// let ids = storage.insert_batch(&vectors)?;
    /// assert_eq!(ids.len(), 2);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn insert_batch(&mut self, vectors: &[SparseVector]) -> Result<Vec<SparseId>, SparseError> {
        if vectors.is_empty() {
            return Ok(Vec::new());
        }

        // Pre-calculate total elements for capacity
        let total_nnz: usize = vectors.iter().map(|v| v.nnz()).sum();

        // Reserve capacity
        self.indices.reserve(total_nnz);
        self.values.reserve(total_nnz);
        self.offsets.reserve(vectors.len());
        self.dims.reserve(vectors.len());

        // Insert each vector
        let mut ids = Vec::with_capacity(vectors.len());
        for vector in vectors {
            let id = self.insert(vector)?;
            ids.push(id);
        }

        Ok(ids)
    }
}
```

**Acceptance Criteria:**
- [ ] Pre-allocates for total elements
- [ ] Returns all IDs in order
- [ ] Empty input returns empty Vec
- [ ] More efficient than single inserts

---

#### W38.2.3: Unit tests for insert (45min)

**File:** `tests/sparse_storage_test.rs`

```rust
use edgevec::sparse::{SparseStorage, SparseVector, SparseId};

#[test]
fn test_insert_single() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();

    let id = storage.insert(&v).unwrap();

    assert_eq!(id.as_u64(), 0);
    assert_eq!(storage.len(), 1);
    assert_eq!(storage.live_count(), 1);
}

#[test]
fn test_insert_multiple_increments_id() {
    let mut storage = SparseStorage::new();

    let v1 = SparseVector::singleton(0, 1.0, 100).unwrap();
    let v2 = SparseVector::singleton(1, 2.0, 100).unwrap();
    let v3 = SparseVector::singleton(2, 3.0, 100).unwrap();

    let id1 = storage.insert(&v1).unwrap();
    let id2 = storage.insert(&v2).unwrap();
    let id3 = storage.insert(&v3).unwrap();

    assert_eq!(id1.as_u64(), 0);
    assert_eq!(id2.as_u64(), 1);
    assert_eq!(id3.as_u64(), 2);
    assert_eq!(storage.len(), 3);
}

#[test]
fn test_insert_batch() {
    let mut storage = SparseStorage::new();
    let vectors: Vec<_> = (0..100)
        .map(|i| SparseVector::singleton(i as u32, i as f32, 1000).unwrap())
        .collect();

    let ids = storage.insert_batch(&vectors).unwrap();

    assert_eq!(ids.len(), 100);
    assert_eq!(storage.len(), 100);

    // Check IDs are sequential
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(id.as_u64(), i as u64);
    }
}

#[test]
fn test_insert_empty_batch() {
    let mut storage = SparseStorage::new();
    let ids = storage.insert_batch(&[]).unwrap();

    assert!(ids.is_empty());
    assert_eq!(storage.len(), 0);
}

#[test]
fn test_insert_preserves_data() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();

    storage.insert(&v).unwrap();

    // Internal arrays should have correct data
    assert_eq!(storage.len(), 1);
}
```

**Acceptance Criteria:**
- [ ] Single insert test passes
- [ ] Multiple inserts increment IDs
- [ ] Batch insert works correctly
- [ ] Empty batch handled
- [ ] Data preserved after insert

---

### Day 2 Checklist

- [ ] W38.2.1: `insert()` method implemented
- [ ] W38.2.2: `insert_batch()` method implemented
- [ ] W38.2.3: Unit tests written and passing
- [ ] `cargo test --features sparse sparse_storage` passes
- [ ] `cargo clippy --features sparse -- -D warnings` passes

### Day 2 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Insert returns unique IDs | Unit tests |
| Data packed correctly | Internal state inspection |
| Batch insert pre-allocates | Code review |
| No memory leaks | Valgrind (optional) |

---

## Day 3: Get Operation + Iterator (3h)

**Date:** 2026-01-21
**Goal:** Retrieve vectors by ID and iterate over storage

### Tasks

#### W38.3.1: Implement `get()` method (1h)

**Objective:** Retrieve sparse vector by ID.

```rust
impl SparseStorage {
    /// Retrieve a sparse vector by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The SparseId to look up
    ///
    /// # Returns
    ///
    /// `Some(SparseVector)` if found and not deleted, `None` otherwise.
    ///
    /// # Complexity
    ///
    /// - Time: O(nnz) - reconstructing the vector
    /// - Space: O(nnz) - new SparseVector allocation
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(5, 1.5, 100)?;
    /// let id = storage.insert(&v)?;
    ///
    /// let retrieved = storage.get(id).unwrap();
    /// assert_eq!(retrieved.indices(), v.indices());
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn get(&self, id: SparseId) -> Option<SparseVector> {
        let idx = id.as_u64() as usize;

        // Bounds check
        if idx >= self.dims.len() {
            return None;
        }

        // Deletion check
        if self.deleted[idx] {
            return None;
        }

        // Get slice boundaries
        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;

        // Reconstruct vector
        let indices = self.indices[start..end].to_vec();
        let values = self.values[start..end].to_vec();
        let dim = self.dims[idx];

        // Safety: data came from validated SparseVector
        Some(SparseVector::new_unchecked(indices, values, dim))
    }

    /// Check if an ID exists and is not deleted.
    #[inline]
    pub fn contains(&self, id: SparseId) -> bool {
        let idx = id.as_u64() as usize;
        idx < self.dims.len() && !self.deleted[idx]
    }

    /// Check if an ID is marked as deleted.
    #[inline]
    pub fn is_deleted(&self, id: SparseId) -> bool {
        let idx = id.as_u64() as usize;
        idx < self.dims.len() && self.deleted[idx]
    }
}
```

**Note:** Add `new_unchecked` to SparseVector for internal use:

```rust
// In src/sparse/vector.rs
impl SparseVector {
    /// Create without validation (internal use only).
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - indices are sorted ascending
    /// - no duplicate indices
    /// - no NaN/Infinity values
    /// - nnz >= 1
    /// - all indices < dim
    #[doc(hidden)]
    pub(crate) fn new_unchecked(indices: Vec<u32>, values: Vec<f32>, dim: u32) -> Self {
        Self { indices, values, dim }
    }
}
```

**Acceptance Criteria:**
- [ ] Returns None for invalid ID
- [ ] Returns None for deleted ID
- [ ] Reconstructs vector correctly
- [ ] O(nnz) time complexity
- [ ] `contains()` helper works

---

#### W38.3.2: Implement iterator over storage (1h)

**Objective:** Iterate over all non-deleted vectors.

```rust
/// Iterator over non-deleted sparse vectors.
pub struct SparseStorageIter<'a> {
    storage: &'a SparseStorage,
    current_idx: usize,
}

impl<'a> Iterator for SparseStorageIter<'a> {
    type Item = (SparseId, SparseVector);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_idx < self.storage.dims.len() {
            let idx = self.current_idx;
            self.current_idx += 1;

            if !self.storage.deleted[idx] {
                let id = SparseId::new(idx as u64);
                if let Some(vector) = self.storage.get(id) {
                    return Some((id, vector));
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.storage.dims.len() - self.current_idx;
        (0, Some(remaining))
    }
}

impl SparseStorage {
    /// Iterate over all non-deleted vectors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// storage.insert(&SparseVector::singleton(0, 1.0, 100)?)?;
    /// storage.insert(&SparseVector::singleton(1, 2.0, 100)?)?;
    ///
    /// for (id, vector) in storage.iter() {
    ///     println!("ID {:?}: {:?}", id, vector);
    /// }
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn iter(&self) -> SparseStorageIter<'_> {
        SparseStorageIter {
            storage: self,
            current_idx: 0,
        }
    }

    /// Iterate over all non-deleted IDs (without reconstructing vectors).
    pub fn ids(&self) -> impl Iterator<Item = SparseId> + '_ {
        (0..self.dims.len())
            .filter(move |&idx| !self.deleted[idx])
            .map(|idx| SparseId::new(idx as u64))
    }
}
```

**Acceptance Criteria:**
- [ ] Iterator skips deleted vectors
- [ ] Returns (SparseId, SparseVector) pairs
- [ ] `ids()` returns only IDs (no vector reconstruction)
- [ ] `size_hint` provides upper bound

---

#### W38.3.3: Unit tests for get and iterator (1h)

**File:** `tests/sparse_storage_test.rs` (continued)

```rust
#[test]
fn test_get_valid() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
    let id = storage.insert(&v).unwrap();

    let retrieved = storage.get(id).unwrap();

    assert_eq!(retrieved.indices(), v.indices());
    assert_eq!(retrieved.values(), v.values());
    assert_eq!(retrieved.dim(), v.dim());
}

#[test]
fn test_get_invalid_id() {
    let storage = SparseStorage::new();
    let result = storage.get(SparseId::new(999));
    assert!(result.is_none());
}

#[test]
fn test_contains() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::singleton(0, 1.0, 100).unwrap();
    let id = storage.insert(&v).unwrap();

    assert!(storage.contains(id));
    assert!(!storage.contains(SparseId::new(999)));
}

#[test]
fn test_iter_all_vectors() {
    let mut storage = SparseStorage::new();

    for i in 0..10 {
        let v = SparseVector::singleton(i as u32, i as f32, 100).unwrap();
        storage.insert(&v).unwrap();
    }

    let collected: Vec<_> = storage.iter().collect();
    assert_eq!(collected.len(), 10);
}

#[test]
fn test_iter_empty_storage() {
    let storage = SparseStorage::new();
    let collected: Vec<_> = storage.iter().collect();
    assert!(collected.is_empty());
}

#[test]
fn test_ids_iterator() {
    let mut storage = SparseStorage::new();

    for i in 0..5 {
        let v = SparseVector::singleton(i as u32, 1.0, 100).unwrap();
        storage.insert(&v).unwrap();
    }

    let ids: Vec<_> = storage.ids().collect();
    assert_eq!(ids.len(), 5);
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(id.as_u64(), i as u64);
    }
}

#[test]
fn test_get_roundtrip_many() {
    let mut storage = SparseStorage::new();

    // Insert various vectors
    let vectors: Vec<_> = (0..100)
        .map(|i| {
            let indices: Vec<u32> = (0..((i % 50) + 1)).map(|j| j * 2).collect();
            let values: Vec<f32> = indices.iter().map(|&j| j as f32 * 0.1).collect();
            SparseVector::new(indices, values, 1000).unwrap()
        })
        .collect();

    let ids: Vec<_> = vectors
        .iter()
        .map(|v| storage.insert(v).unwrap())
        .collect();

    // Verify all roundtrip correctly
    for (id, original) in ids.iter().zip(vectors.iter()) {
        let retrieved = storage.get(*id).unwrap();
        assert_eq!(retrieved.indices(), original.indices());
        assert_eq!(retrieved.values(), original.values());
        assert_eq!(retrieved.dim(), original.dim());
    }
}
```

**Acceptance Criteria:**
- [ ] Get returns correct data
- [ ] Get returns None for invalid ID
- [ ] Iterator returns all non-deleted vectors
- [ ] IDs iterator efficient (no vector reconstruction)
- [ ] Roundtrip test passes for 100 vectors

---

### Day 3 Checklist

- [ ] W38.3.1: `get()` and helpers implemented
- [ ] W38.3.2: Iterator implemented
- [ ] W38.3.3: Unit tests passing
- [ ] `new_unchecked()` added to SparseVector
- [ ] `cargo test --features sparse` passes
- [ ] `cargo clippy --features sparse -- -D warnings` passes

### Day 3 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Get roundtrip preserves data | Unit tests |
| Iterator skips deleted | Unit tests (Day 5) |
| O(nnz) retrieval | Code review |
| Contains helper O(1) | Code review |

---

## Day 4: Serialization (3h)

**Date:** 2026-01-22
**Goal:** Implement save/load for SparseStorage

### Tasks

#### W38.4.1: Define serialization format (30min)

**Objective:** Design binary format matching existing dense storage pattern.

**Format Specification:**

```
SPARSE STORAGE BINARY FORMAT v1

Header (24 bytes):
  [0..8]   Magic: "EDGSPRSE" (8 bytes)
  [8..12]  Version: u32 = 1
  [12..16] Flags: u32 (reserved)
  [16..20] Vector count: u32
  [20..24] Total elements: u32

Offsets section (vector_count + 1 * 4 bytes):
  [u32; vector_count + 1] offsets

Dims section (vector_count * 4 bytes):
  [u32; vector_count] dims

Deleted bitmap (ceil(vector_count / 8) bytes):
  [u8; ceil(vector_count / 8)] deleted bits

Indices section (total_elements * 4 bytes):
  [u32; total_elements] indices

Values section (total_elements * 4 bytes):
  [f32; total_elements] values

Footer (8 bytes):
  [u64] CRC64 checksum
```

**Acceptance Criteria:**
- [ ] Format documented
- [ ] Magic number unique to sparse storage
- [ ] Version field for future compatibility
- [ ] Checksum for integrity

---

#### W38.4.2: Implement `save()` method (1h)

**Objective:** Serialize storage to bytes.

```rust
impl SparseStorage {
    /// Magic bytes for sparse storage files.
    const MAGIC: &'static [u8; 8] = b"EDGSPRSE";
    /// Current format version.
    const VERSION: u32 = 1;

    /// Serialize storage to bytes.
    ///
    /// # Returns
    ///
    /// Byte vector containing the complete storage.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// storage.insert(&SparseVector::singleton(0, 1.0, 100)?)?;
    ///
    /// let bytes = storage.save();
    /// // bytes can be written to file or IndexedDB
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn save(&self) -> Vec<u8> {
        let vector_count = self.dims.len() as u32;
        let total_elements = self.indices.len() as u32;

        // Calculate total size
        let header_size = 24;
        let offsets_size = (vector_count + 1) as usize * 4;
        let dims_size = vector_count as usize * 4;
        let deleted_size = (vector_count as usize + 7) / 8;
        let indices_size = total_elements as usize * 4;
        let values_size = total_elements as usize * 4;
        let footer_size = 8;

        let total_size = header_size + offsets_size + dims_size + deleted_size
            + indices_size + values_size + footer_size;

        let mut buffer = Vec::with_capacity(total_size);

        // Header
        buffer.extend_from_slice(Self::MAGIC);
        buffer.extend_from_slice(&Self::VERSION.to_le_bytes());
        buffer.extend_from_slice(&0u32.to_le_bytes()); // Flags (reserved)
        buffer.extend_from_slice(&vector_count.to_le_bytes());
        buffer.extend_from_slice(&total_elements.to_le_bytes());

        // Offsets
        for offset in &self.offsets {
            buffer.extend_from_slice(&offset.to_le_bytes());
        }

        // Dims
        for dim in &self.dims {
            buffer.extend_from_slice(&dim.to_le_bytes());
        }

        // Deleted bitmap
        let deleted_bytes = self.deleted.as_raw_slice();
        buffer.extend_from_slice(deleted_bytes);
        // Pad to byte boundary
        let padding = deleted_size - deleted_bytes.len();
        buffer.extend(std::iter::repeat(0u8).take(padding));

        // Indices
        for idx in &self.indices {
            buffer.extend_from_slice(&idx.to_le_bytes());
        }

        // Values
        for val in &self.values {
            buffer.extend_from_slice(&val.to_le_bytes());
        }

        // Checksum (CRC64 of everything before footer)
        let checksum = crc64(&buffer);
        buffer.extend_from_slice(&checksum.to_le_bytes());

        buffer
    }
}

/// CRC64 checksum (placeholder - use crc crate in production)
fn crc64(data: &[u8]) -> u64 {
    // Simple implementation - replace with crc crate
    let mut hash: u64 = 0;
    for byte in data {
        hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
    }
    hash
}
```

**Acceptance Criteria:**
- [ ] Magic number written first
- [ ] Version field written
- [ ] All data sections serialized
- [ ] Checksum computed and appended
- [ ] Output size matches calculation

---

#### W38.4.3: Implement `load()` method (1h)

**Objective:** Deserialize storage from bytes.

```rust
impl SparseStorage {
    /// Deserialize storage from bytes.
    ///
    /// # Arguments
    ///
    /// * `data` - Byte slice containing serialized storage
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Magic number invalid
    /// - Version unsupported
    /// - Checksum mismatch
    /// - Data truncated
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// storage.insert(&SparseVector::singleton(0, 1.0, 100)?)?;
    /// let bytes = storage.save();
    ///
    /// let loaded = SparseStorage::load(&bytes)?;
    /// assert_eq!(loaded.len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn load(data: &[u8]) -> Result<Self, SparseError> {
        if data.len() < 24 {
            return Err(SparseError::InvalidFormat("data too short".into()));
        }

        // Validate magic
        if &data[0..8] != Self::MAGIC {
            return Err(SparseError::InvalidFormat("invalid magic".into()));
        }

        // Read header
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        if version != Self::VERSION {
            return Err(SparseError::InvalidFormat(
                format!("unsupported version: {}", version)
            ));
        }

        let _flags = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let vector_count = u32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;
        let total_elements = u32::from_le_bytes(data[20..24].try_into().unwrap()) as usize;

        // Calculate expected sizes
        let header_size = 24;
        let offsets_size = (vector_count + 1) * 4;
        let dims_size = vector_count * 4;
        let deleted_size = (vector_count + 7) / 8;
        let indices_size = total_elements * 4;
        let values_size = total_elements * 4;
        let footer_size = 8;

        let expected_size = header_size + offsets_size + dims_size + deleted_size
            + indices_size + values_size + footer_size;

        if data.len() != expected_size {
            return Err(SparseError::InvalidFormat(
                format!("size mismatch: expected {}, got {}", expected_size, data.len())
            ));
        }

        // Validate checksum
        let stored_checksum = u64::from_le_bytes(
            data[data.len() - 8..].try_into().unwrap()
        );
        let computed_checksum = crc64(&data[..data.len() - 8]);
        if stored_checksum != computed_checksum {
            return Err(SparseError::InvalidFormat("checksum mismatch".into()));
        }

        // Parse sections
        let mut pos = header_size;

        // Offsets
        let mut offsets = Vec::with_capacity(vector_count + 1);
        for _ in 0..=vector_count {
            let offset = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
            offsets.push(offset);
            pos += 4;
        }

        // Dims
        let mut dims = Vec::with_capacity(vector_count);
        for _ in 0..vector_count {
            let dim = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
            dims.push(dim);
            pos += 4;
        }

        // Deleted bitmap
        let deleted_bytes = &data[pos..pos + deleted_size];
        let mut deleted = BitVec::with_capacity(vector_count);
        for i in 0..vector_count {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            let is_deleted = (deleted_bytes[byte_idx] >> bit_idx) & 1 == 1;
            deleted.push(is_deleted);
        }
        pos += deleted_size;

        // Indices
        let mut indices = Vec::with_capacity(total_elements);
        for _ in 0..total_elements {
            let idx = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
            indices.push(idx);
            pos += 4;
        }

        // Values
        let mut values = Vec::with_capacity(total_elements);
        for _ in 0..total_elements {
            let val = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
            values.push(val);
            pos += 4;
        }

        // Compute next_id
        let next_id = vector_count as u64;

        Ok(Self {
            indices,
            values,
            offsets,
            dims,
            deleted,
            next_id,
        })
    }
}
```

**Note:** Add new error variant to SparseError:

```rust
// In src/sparse/error.rs
#[derive(Error, Debug)]
pub enum SparseError {
    // ... existing variants ...

    #[error("invalid storage format: {0}")]
    InvalidFormat(String),
}
```

**Acceptance Criteria:**
- [ ] Validates magic number
- [ ] Validates version
- [ ] Validates checksum
- [ ] Validates data length
- [ ] Reconstructs all fields correctly
- [ ] Returns descriptive errors

---

#### W38.4.4: Serialization tests (30min)

**File:** `tests/sparse_storage_test.rs` (continued)

```rust
#[test]
fn test_save_load_empty() {
    let storage = SparseStorage::new();
    let bytes = storage.save();
    let loaded = SparseStorage::load(&bytes).unwrap();

    assert_eq!(loaded.len(), 0);
}

#[test]
fn test_save_load_single() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
    storage.insert(&v).unwrap();

    let bytes = storage.save();
    let loaded = SparseStorage::load(&bytes).unwrap();

    assert_eq!(loaded.len(), 1);
    let retrieved = loaded.get(SparseId::new(0)).unwrap();
    assert_eq!(retrieved.indices(), v.indices());
    assert_eq!(retrieved.values(), v.values());
}

#[test]
fn test_save_load_many() {
    let mut storage = SparseStorage::new();

    for i in 0..100 {
        let v = SparseVector::singleton(i as u32, i as f32 * 0.1, 1000).unwrap();
        storage.insert(&v).unwrap();
    }

    let bytes = storage.save();
    let loaded = SparseStorage::load(&bytes).unwrap();

    assert_eq!(loaded.len(), 100);
    assert_eq!(loaded.live_count(), 100);
}

#[test]
fn test_load_invalid_magic() {
    let mut data = b"WRONGMAG".to_vec();
    data.extend([0u8; 100]);

    let result = SparseStorage::load(&data);
    assert!(result.is_err());
}

#[test]
fn test_load_invalid_checksum() {
    let mut storage = SparseStorage::new();
    storage.insert(&SparseVector::singleton(0, 1.0, 100).unwrap()).unwrap();

    let mut bytes = storage.save();
    // Corrupt last byte (part of checksum)
    *bytes.last_mut().unwrap() ^= 0xFF;

    let result = SparseStorage::load(&bytes);
    assert!(result.is_err());
}

#[test]
fn test_load_truncated() {
    let mut storage = SparseStorage::new();
    storage.insert(&SparseVector::singleton(0, 1.0, 100).unwrap()).unwrap();

    let bytes = storage.save();
    let truncated = &bytes[..bytes.len() - 10];

    let result = SparseStorage::load(truncated);
    assert!(result.is_err());
}
```

**Acceptance Criteria:**
- [ ] Empty storage roundtrips
- [ ] Single vector roundtrips
- [ ] Many vectors roundtrip
- [ ] Invalid magic rejected
- [ ] Invalid checksum rejected
- [ ] Truncated data rejected

---

### Day 4 Checklist

- [ ] W38.4.1: Format documented
- [ ] W38.4.2: `save()` implemented
- [ ] W38.4.3: `load()` implemented
- [ ] W38.4.4: Serialization tests passing
- [ ] `InvalidFormat` error variant added
- [ ] `cargo test --features sparse` passes
- [ ] `cargo clippy --features sparse -- -D warnings` passes

### Day 4 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Save produces valid bytes | Unit tests |
| Load restores all data | Unit tests |
| Invalid data rejected | Error tests |
| Checksum catches corruption | Corruption test |

---

## Day 5: Deletion Support (3h)

**Date:** 2026-01-23
**Goal:** Implement soft delete with BitVec

### Tasks

#### W38.5.1: Implement `delete()` method (1h)

**Objective:** Mark vectors as deleted.

```rust
impl SparseStorage {
    /// Mark a vector as deleted.
    ///
    /// Performs soft deletion - the vector's data remains in storage
    /// but is marked as deleted and excluded from iteration and get.
    ///
    /// # Arguments
    ///
    /// * `id` - The SparseId to delete
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if vector was deleted
    /// - `Ok(false)` if vector was already deleted
    /// - `Err` if ID not found
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// let id = storage.insert(&v)?;
    ///
    /// assert!(storage.contains(id));
    /// storage.delete(id)?;
    /// assert!(!storage.contains(id));
    /// assert!(storage.is_deleted(id));
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn delete(&mut self, id: SparseId) -> Result<bool, SparseError> {
        let idx = id.as_u64() as usize;

        // Bounds check
        if idx >= self.dims.len() {
            return Err(SparseError::IdNotFound(id.as_u64()));
        }

        // Check if already deleted
        if self.deleted[idx] {
            return Ok(false);
        }

        // Mark as deleted
        self.deleted.set(idx, true);

        Ok(true)
    }

    /// Restore a deleted vector.
    ///
    /// # Arguments
    ///
    /// * `id` - The SparseId to restore
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if vector was restored
    /// - `Ok(false)` if vector was not deleted
    /// - `Err` if ID not found
    pub fn restore(&mut self, id: SparseId) -> Result<bool, SparseError> {
        let idx = id.as_u64() as usize;

        // Bounds check
        if idx >= self.dims.len() {
            return Err(SparseError::IdNotFound(id.as_u64()));
        }

        // Check if not deleted
        if !self.deleted[idx] {
            return Ok(false);
        }

        // Restore
        self.deleted.set(idx, false);

        Ok(true)
    }
}
```

**Acceptance Criteria:**
- [ ] Returns error for invalid ID
- [ ] Returns true on successful delete
- [ ] Returns false if already deleted
- [ ] Deleted vectors excluded from `get()`
- [ ] `restore()` can undo deletion

---

#### W38.5.2: Implement `delete_batch()` method (30min)

**Objective:** Delete multiple vectors efficiently.

```rust
impl SparseStorage {
    /// Delete multiple vectors in a batch.
    ///
    /// # Arguments
    ///
    /// * `ids` - Slice of SparseIds to delete
    ///
    /// # Returns
    ///
    /// Number of vectors actually deleted (excludes already-deleted).
    ///
    /// # Errors
    ///
    /// Returns error if any ID is not found. Deletion is atomic:
    /// if any ID is invalid, no deletions are performed.
    pub fn delete_batch(&mut self, ids: &[SparseId]) -> Result<usize, SparseError> {
        // Validate all IDs first
        for id in ids {
            let idx = id.as_u64() as usize;
            if idx >= self.dims.len() {
                return Err(SparseError::IdNotFound(id.as_u64()));
            }
        }

        // Perform deletions
        let mut deleted_count = 0;
        for id in ids {
            let idx = id.as_u64() as usize;
            if !self.deleted[idx] {
                self.deleted.set(idx, true);
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }
}
```

**Acceptance Criteria:**
- [ ] Validates all IDs before deleting
- [ ] Atomic: all-or-nothing on invalid ID
- [ ] Returns count of newly deleted
- [ ] Efficient for large batches

---

#### W38.5.3: Verify iterator skips deleted (30min)

**Objective:** Ensure deleted vectors excluded from iteration.

Add tests to verify Day 3's iterator implementation:

```rust
#[test]
fn test_iter_skips_deleted() {
    let mut storage = SparseStorage::new();

    // Insert 5 vectors
    for i in 0..5 {
        let v = SparseVector::singleton(i as u32, i as f32, 100).unwrap();
        storage.insert(&v).unwrap();
    }

    // Delete IDs 1 and 3
    storage.delete(SparseId::new(1)).unwrap();
    storage.delete(SparseId::new(3)).unwrap();

    // Verify iterator returns only 0, 2, 4
    let collected: Vec<_> = storage.iter().map(|(id, _)| id.as_u64()).collect();
    assert_eq!(collected, vec![0, 2, 4]);
    assert_eq!(storage.live_count(), 3);
}

#[test]
fn test_ids_iterator_skips_deleted() {
    let mut storage = SparseStorage::new();

    for i in 0..5 {
        let v = SparseVector::singleton(i as u32, 1.0, 100).unwrap();
        storage.insert(&v).unwrap();
    }

    storage.delete(SparseId::new(0)).unwrap();
    storage.delete(SparseId::new(4)).unwrap();

    let ids: Vec<_> = storage.ids().map(|id| id.as_u64()).collect();
    assert_eq!(ids, vec![1, 2, 3]);
}

#[test]
fn test_get_returns_none_for_deleted() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::singleton(0, 1.0, 100).unwrap();
    let id = storage.insert(&v).unwrap();

    // Before delete: get works
    assert!(storage.get(id).is_some());

    // After delete: get returns None
    storage.delete(id).unwrap();
    assert!(storage.get(id).is_none());

    // Restore and verify get works again
    storage.restore(id).unwrap();
    assert!(storage.get(id).is_some());
}
```

**Acceptance Criteria:**
- [ ] `iter()` skips deleted vectors
- [ ] `ids()` skips deleted IDs
- [ ] `get()` returns None for deleted
- [ ] `live_count()` reflects deletions
- [ ] Restore makes vector accessible again

---

#### W38.5.4: Deletion tests (1h)

**File:** `tests/sparse_storage_test.rs` (continued)

```rust
#[test]
fn test_delete_valid() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::singleton(0, 1.0, 100).unwrap();
    let id = storage.insert(&v).unwrap();

    let result = storage.delete(id).unwrap();
    assert!(result); // Was deleted
    assert!(storage.is_deleted(id));
    assert!(!storage.contains(id));
}

#[test]
fn test_delete_already_deleted() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::singleton(0, 1.0, 100).unwrap();
    let id = storage.insert(&v).unwrap();

    storage.delete(id).unwrap();
    let result = storage.delete(id).unwrap();
    assert!(!result); // Already deleted
}

#[test]
fn test_delete_invalid_id() {
    let mut storage = SparseStorage::new();
    let result = storage.delete(SparseId::new(999));
    assert!(result.is_err());
}

#[test]
fn test_restore_deleted() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::singleton(0, 1.0, 100).unwrap();
    let id = storage.insert(&v).unwrap();

    storage.delete(id).unwrap();
    assert!(!storage.contains(id));

    let restored = storage.restore(id).unwrap();
    assert!(restored);
    assert!(storage.contains(id));
}

#[test]
fn test_restore_not_deleted() {
    let mut storage = SparseStorage::new();
    let v = SparseVector::singleton(0, 1.0, 100).unwrap();
    let id = storage.insert(&v).unwrap();

    let restored = storage.restore(id).unwrap();
    assert!(!restored); // Was not deleted
}

#[test]
fn test_delete_batch_atomic() {
    let mut storage = SparseStorage::new();

    for i in 0..5 {
        let v = SparseVector::singleton(i as u32, 1.0, 100).unwrap();
        storage.insert(&v).unwrap();
    }

    // Try to delete with invalid ID - should fail atomically
    let result = storage.delete_batch(&[
        SparseId::new(0),
        SparseId::new(1),
        SparseId::new(999), // Invalid
    ]);

    assert!(result.is_err());
    // All vectors should still exist
    assert_eq!(storage.live_count(), 5);
}

#[test]
fn test_delete_batch_success() {
    let mut storage = SparseStorage::new();

    for i in 0..5 {
        let v = SparseVector::singleton(i as u32, 1.0, 100).unwrap();
        storage.insert(&v).unwrap();
    }

    let deleted = storage.delete_batch(&[
        SparseId::new(0),
        SparseId::new(2),
        SparseId::new(4),
    ]).unwrap();

    assert_eq!(deleted, 3);
    assert_eq!(storage.live_count(), 2);
}

#[test]
fn test_save_load_preserves_deleted() {
    let mut storage = SparseStorage::new();

    for i in 0..5 {
        let v = SparseVector::singleton(i as u32, i as f32, 100).unwrap();
        storage.insert(&v).unwrap();
    }

    storage.delete(SparseId::new(1)).unwrap();
    storage.delete(SparseId::new(3)).unwrap();

    let bytes = storage.save();
    let loaded = SparseStorage::load(&bytes).unwrap();

    assert_eq!(loaded.len(), 5);
    assert_eq!(loaded.live_count(), 3);
    assert!(loaded.is_deleted(SparseId::new(1)));
    assert!(loaded.is_deleted(SparseId::new(3)));
    assert!(!loaded.is_deleted(SparseId::new(0)));
    assert!(!loaded.is_deleted(SparseId::new(2)));
    assert!(!loaded.is_deleted(SparseId::new(4)));
}
```

**Acceptance Criteria:**
- [ ] Delete works for valid ID
- [ ] Delete returns false for already-deleted
- [ ] Delete returns error for invalid ID
- [ ] Restore works correctly
- [ ] Batch delete is atomic
- [ ] Deletion state persists through save/load

---

### Day 5 Checklist

- [ ] W38.5.1: `delete()` and `restore()` implemented
- [ ] W38.5.2: `delete_batch()` implemented
- [ ] W38.5.3: Iterator skip-deleted verified
- [ ] W38.5.4: Deletion tests passing
- [ ] Save/load preserves deleted state
- [ ] `cargo test --features sparse` passes
- [ ] `cargo clippy --features sparse -- -D warnings` passes

### Day 5 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Delete marks as deleted | Unit tests |
| Restore undoes deletion | Unit tests |
| Batch delete atomic | Error test |
| Deleted state persists | Serialization test |
| Iterator excludes deleted | Iteration tests |

---

## Day 6: Benchmarks + Hostile Review (2h)

**Date:** 2026-01-24
**Goal:** Validate performance targets and get hostile review

### Tasks

#### W38.6.1: Create benchmark file (45min)

**File:** `benches/sparse_storage_bench.rs`

```rust
//! Sparse storage benchmarks.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use edgevec::sparse::{SparseStorage, SparseVector, SparseId};

fn random_sparse(dim: u32, nnz: usize, seed: u64) -> SparseVector {
    use std::collections::HashSet;
    let mut indices: HashSet<u32> = HashSet::with_capacity(nnz);
    let mut rng_state = seed;

    while indices.len() < nnz {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (rng_state >> 33) as u32 % dim;
        indices.insert(idx);
    }

    let mut indices: Vec<_> = indices.into_iter().collect();
    indices.sort_unstable();

    let values: Vec<f32> = indices.iter().enumerate()
        .map(|(i, _)| ((i as f32 + 1.0) * 0.1) % 1.0)
        .collect();

    SparseVector::new(indices, values, dim).unwrap()
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_storage_insert");

    for nnz in [10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::new("single", nnz),
            &nnz,
            |b, &nnz| {
                let v = random_sparse(10000, nnz, 42);
                b.iter(|| {
                    let mut storage = SparseStorage::new();
                    black_box(storage.insert(&v).unwrap())
                });
            }
        );
    }

    group.finish();
}

fn bench_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_storage_get");

    for n in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("by_id", n),
            &n,
            |b, &n| {
                let mut storage = SparseStorage::with_capacity(n, 50);
                for i in 0..n {
                    let v = random_sparse(10000, 50, i as u64);
                    storage.insert(&v).unwrap();
                }

                let mid_id = SparseId::new((n / 2) as u64);
                b.iter(|| black_box(storage.get(mid_id).unwrap()));
            }
        );
    }

    group.finish();
}

fn bench_iterate(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_storage_iter");

    for n in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("all", n),
            &n,
            |b, &n| {
                let mut storage = SparseStorage::with_capacity(n, 50);
                for i in 0..n {
                    let v = random_sparse(10000, 50, i as u64);
                    storage.insert(&v).unwrap();
                }

                b.iter(|| {
                    let count = storage.iter().count();
                    black_box(count)
                });
            }
        );
    }

    group.finish();
}

fn bench_save_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_storage_serialize");

    for n in [1_000, 10_000] {
        let mut storage = SparseStorage::with_capacity(n, 50);
        for i in 0..n {
            let v = random_sparse(10000, 50, i as u64);
            storage.insert(&v).unwrap();
        }

        group.bench_with_input(
            BenchmarkId::new("save", n),
            &storage,
            |b, storage| {
                b.iter(|| black_box(storage.save()));
            }
        );

        let bytes = storage.save();
        group.bench_with_input(
            BenchmarkId::new("load", n),
            &bytes,
            |b, bytes| {
                b.iter(|| black_box(SparseStorage::load(bytes).unwrap()));
            }
        );
    }

    group.finish();
}

fn bench_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_storage_delete");

    for n in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("single", n),
            &n,
            |b, &n| {
                b.iter_batched(
                    || {
                        let mut storage = SparseStorage::with_capacity(n, 50);
                        for i in 0..n {
                            let v = random_sparse(10000, 50, i as u64);
                            storage.insert(&v).unwrap();
                        }
                        storage
                    },
                    |mut storage| {
                        black_box(storage.delete(SparseId::new((n / 2) as u64)).unwrap())
                    },
                    criterion::BatchSize::SmallInput,
                );
            }
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_get,
    bench_iterate,
    bench_save_load,
    bench_delete,
);
criterion_main!(benches);
```

**Acceptance Criteria:**
- [ ] Insert benchmark for various nnz
- [ ] Get benchmark for various storage sizes
- [ ] Iterate benchmark
- [ ] Serialize benchmark
- [ ] Delete benchmark

---

#### W38.6.2: Run benchmarks and document results (30min)

**Objective:** Run benchmarks and compare to RFC-007 targets.

```bash
cargo bench --bench sparse_storage_bench --features sparse
```

**Performance Targets (from RFC-007):**

| Operation | P50 Target | P99 Target |
|:----------|:-----------|:-----------|
| Insert (50 nnz) | <50us | <100us |
| Get (50 nnz) | <5us | <10us |
| Delete | <1us | <5us |

**Results Template:**

```markdown
## Week 38 Benchmark Results

Date: 2026-01-24
Hardware: [CPU, RAM]

### Insert Performance

| nnz | P50 | P99 | vs Target |
|:----|:----|:----|:----------|
| 10  | Xus | Xus | PASS/FAIL |
| 50  | Xus | Xus | PASS/FAIL |
| 100 | Xus | Xus | N/A |
| 500 | Xus | Xus | N/A |

### Get Performance (50 nnz, 10k vectors)

| Storage Size | P50 | P99 | vs Target |
|:-------------|:----|:----|:----------|
| 1k  | Xus | Xus | PASS/FAIL |
| 10k | Xus | Xus | PASS/FAIL |
| 100k | Xus | Xus | N/A |

### Serialization Performance

| Operation | 1k vectors | 10k vectors |
|:----------|:-----------|:------------|
| Save | Xms | Xms |
| Load | Xms | Xms |
```

**Acceptance Criteria:**
- [ ] Benchmarks run without error
- [ ] Results documented
- [ ] Comparison to RFC-007 targets
- [ ] Performance acceptable (no > 3x target violations)

---

#### W38.6.3: Submit for hostile review (45min)

**Objective:** Get HOSTILE_REVIEWER approval for Week 38 deliverables.

**Review Command:**
```
/review src/sparse/storage.rs tests/sparse_storage_test.rs benches/sparse_storage_bench.rs
```

**Review Checklist:**

- [ ] `SparseStorage` matches RFC-007 design
- [ ] All methods have doc comments
- [ ] Error handling comprehensive
- [ ] Serialization format documented
- [ ] Checksum protects integrity
- [ ] Deletion is soft (no data loss)
- [ ] Tests cover edge cases
- [ ] Benchmarks validate performance
- [ ] No clippy warnings
- [ ] No unsafe code (or justified)

**Acceptance Criteria:**
- [ ] HOSTILE_REVIEWER issues identified
- [ ] All CRITICAL issues addressed
- [ ] All MAJOR issues addressed
- [ ] Review verdict: APPROVED or CONDITIONAL_APPROVAL

---

### Day 6 Checklist

- [ ] W38.6.1: Benchmark file created
- [ ] W38.6.2: Benchmarks run, results documented
- [ ] W38.6.3: Hostile review submitted
- [ ] All CRITICAL/MAJOR issues addressed
- [ ] Review document created: `docs/reviews/2026-01-24_sparse_storage_APPROVED.md`

### Day 6 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Insert P50 < 50us (50 nnz) | Benchmark |
| Get P50 < 5us (50 nnz) | Benchmark |
| Hostile review APPROVED | Review document |
| All tests pass | `cargo test` |

---

## Week 38 Deliverables

### Files Created

```
src/sparse/
└── storage.rs          # SparseStorage implementation (updated)

tests/
└── sparse_storage_test.rs    # Storage unit and property tests

benches/
└── sparse_storage_bench.rs   # Storage performance benchmarks

docs/reviews/
└── 2026-01-24_sparse_storage_APPROVED.md   # Hostile review
```

### Files Modified

```
src/sparse/mod.rs       # Add storage exports
src/sparse/error.rs     # Add InvalidFormat variant
src/sparse/vector.rs    # Add new_unchecked (internal)
```

---

## Week 38 Risk Register

| ID | Risk | Likelihood | Impact | Mitigation |
|:---|:-----|:-----------|:-------|:-----------|
| R38.1 | Insert performance exceeds target | LOW | HIGH | Use pre-allocated capacity |
| R38.2 | Serialization format issues | LOW | MEDIUM | Follow existing dense storage patterns |
| R38.3 | BitVec API incompatibility | LOW | LOW | bitvec crate already in Cargo.toml |
| R38.4 | Checksum collision | VERY LOW | LOW | CRC64 has low collision rate |
| R38.5 | ID overflow | VERY LOW | HIGH | u64 supports 18 quintillion IDs |

---

## Week 38 Exit Criteria

Week 38 is complete when:
- [ ] `SparseStorage` fully implemented
- [ ] Insert, get, delete operations working
- [ ] Serialization (save/load) working
- [ ] Deletion state persists through serialization
- [ ] Benchmarks meet RFC-007 targets (or within 2x)
- [ ] All unit tests pass
- [ ] HOSTILE_REVIEWER APPROVED
- [ ] Week 39 plan created

---

## Week 38 Handoff

### To Week 39

**Completed:**
- `SparseStorage` with packed arrays
- Insert, get, delete operations
- Serialization with checksum
- Property tests and benchmarks

**Ready for Week 39:**
- `SparseSearcher` implementation (brute-force)
- Integration with `SparseStorage`
- Search benchmarks

**Dependencies for Week 39:**
- `SparseStorage::iter()` for brute-force search
- `sparse_dot_product()` from Week 37
- `SearchResult` type from existing codebase

---

## Commit Message Template

```
feat(sparse): implement SparseStorage with persistence (Week 38)

- Add SparseStorage with packed indices/values arrays
- Implement insert, get, delete operations
- Add binary serialization with checksum
- Property tests verify roundtrip correctness
- Benchmarks validate <100us insert (50 nnz)

RFC-007 Phase 2 complete. Ready for Phase 3 (Search).

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

**Agent:** PLANNER
**Hours:** 16h total
**Priority:** P0 (v0.9.0 core feature)
**Status:** [PROPOSED]

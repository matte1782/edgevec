# Week 40 Day 1: Foundation & Data Structure

**Date:** 2026-02-03
**Focus:** Core FlatIndex type, memory layout, and vector insertion
**Estimated Duration:** 5 hours
**Phase:** RFC-008 Phase 1 (Foundation)
**Dependencies:** Week 39 COMPLETE

---

## Context

Day 1 establishes the foundation for FlatIndex - a brute-force search index optimized for small datasets (<10k vectors) where 100% recall is required.

**Architecture Reference:**
- Memory layout: Column-major F32 for cache-friendly SIMD access
- ID allocation: Sequential u64 with deleted bitmap
- Metric support: Cosine, Dot, L2, Hamming (same as HNSW)

**Why FlatIndex:**
- HNSW has graph construction overhead (~40% extra memory)
- For small datasets, brute-force is fast enough (<50ms at 10k)
- Exact recall (100%) required for precision-critical applications
- O(1) insert vs O(log n) for HNSW

---

## Tasks

### W40.1.1: Create Module Structure

**Objective:** Set up the index module with FlatIndex struct definition.

**File:** `src/index/mod.rs`

```rust
//! Index implementations for EdgeVec.
//!
//! This module provides different indexing strategies:
//! - [`FlatIndex`]: Brute-force search for small datasets (<10k vectors)
//! - Future: HNSW integration, IVF, etc.

mod flat;

pub use flat::{FlatIndex, FlatIndexConfig};
```

**File:** `src/index/flat.rs`

```rust
//! Flat (brute-force) index for exact nearest neighbor search.
//!
//! # Overview
//!
//! `FlatIndex` stores vectors in a contiguous memory layout and performs
//! exhaustive distance computation during search. This provides:
//!
//! - **100% recall**: Every vector is compared, guaranteeing exact results
//! - **O(1) insert**: Vectors are appended without graph construction
//! - **Low memory overhead**: No graph structure, just vectors + bitmap
//!
//! # Use Cases
//!
//! Best suited for:
//! - Small datasets (<10,000 vectors)
//! - Precision-critical applications requiring exact results
//! - Append-heavy workloads (real-time embeddings)
//! - Binary vector search with Hamming distance
//!
//! # Example
//!
//! ```rust
//! use edgevec::index::{FlatIndex, FlatIndexConfig};
//! use edgevec::metric::Metric;
//!
//! // Create a flat index for 128-dimensional vectors
//! let config = FlatIndexConfig::new(128).with_metric(Metric::Cosine);
//! let mut index = FlatIndex::new(config);
//!
//! // Insert vectors (O(1) operation)
//! let id1 = index.insert(&[0.1; 128])?;
//! let id2 = index.insert(&[0.2; 128])?;
//!
//! // Search (brute-force, 100% recall)
//! let results = index.search(&[0.15; 128], 10)?;
//! ```

use crate::error::IndexError;
use crate::metric::Metric;
use bitvec::prelude::*;

/// Configuration for FlatIndex.
#[derive(Debug, Clone)]
pub struct FlatIndexConfig {
    /// Vector dimension
    pub dimensions: u32,

    /// Distance metric
    pub metric: Metric,

    /// Initial capacity (number of vectors to pre-allocate)
    pub initial_capacity: usize,

    /// Cleanup threshold (fraction of deleted vectors before compaction)
    pub cleanup_threshold: f32,
}

impl FlatIndexConfig {
    /// Create a new configuration with the given dimensions.
    pub fn new(dimensions: u32) -> Self {
        Self {
            dimensions,
            metric: Metric::Cosine,
            initial_capacity: 1000,
            cleanup_threshold: 0.5,
        }
    }

    /// Set the distance metric.
    pub fn with_metric(mut self, metric: Metric) -> Self {
        self.metric = metric;
        self
    }

    /// Set the initial capacity.
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.initial_capacity = capacity;
        self
    }

    /// Set the cleanup threshold (0.0 to 1.0).
    pub fn with_cleanup_threshold(mut self, threshold: f32) -> Self {
        self.cleanup_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

/// Flat (brute-force) index for exact nearest neighbor search.
///
/// Stores vectors in column-major layout for cache-friendly SIMD access.
/// Provides O(1) insertion and O(n*d) search with 100% recall guarantee.
pub struct FlatIndex {
    /// Configuration
    config: FlatIndexConfig,

    /// Dense vectors in column-major layout: [v0_d0, v1_d0, ..., vn_d0, v0_d1, ...]
    /// This layout is optimal for SIMD distance computation across vectors
    vectors: Vec<f32>,

    /// Number of vectors stored (including deleted)
    count: u64,

    /// Bitmap tracking deleted vectors
    deleted: BitVec,

    /// Number of deleted vectors (for cleanup threshold)
    delete_count: usize,

    /// Next ID to assign
    next_id: u64,
}

impl FlatIndex {
    /// Create a new FlatIndex with the given configuration.
    pub fn new(config: FlatIndexConfig) -> Self {
        let capacity = config.initial_capacity;
        let dim = config.dimensions as usize;

        Self {
            config,
            vectors: Vec::with_capacity(capacity * dim),
            count: 0,
            deleted: BitVec::with_capacity(capacity),
            delete_count: 0,
            next_id: 0,
        }
    }

    /// Returns the vector dimension.
    pub fn dimensions(&self) -> u32 {
        self.config.dimensions
    }

    /// Returns the distance metric.
    pub fn metric(&self) -> Metric {
        self.config.metric
    }

    /// Returns the number of vectors (excluding deleted).
    pub fn len(&self) -> usize {
        self.count as usize - self.delete_count
    }

    /// Returns true if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total capacity (including deleted slots).
    pub fn capacity(&self) -> usize {
        self.count as usize
    }
}
```

**Acceptance Criteria:**
- [ ] `src/index/mod.rs` created with module exports
- [ ] `src/index/flat.rs` created with FlatIndex struct
- [ ] FlatIndexConfig with builder pattern
- [ ] Doc comments with examples
- [ ] Compiles without errors

**Deliverables:**
- `src/index/mod.rs`
- `src/index/flat.rs`

**Dependencies:** None

**Estimated Duration:** 1.5 hours

**Agent:** RUST_ENGINEER

---

### W40.1.2: Implement Insert Method

**Objective:** Implement vector insertion with ID allocation and storage.

**File:** `src/index/flat.rs` (continued)

```rust
impl FlatIndex {
    /// Insert a vector into the index.
    ///
    /// Returns the assigned vector ID.
    ///
    /// # Errors
    ///
    /// Returns `IndexError::DimensionMismatch` if the vector dimension
    /// doesn't match the index configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(128));
    /// let id = index.insert(&[0.1; 128])?;
    /// assert_eq!(id, 0);
    /// ```
    pub fn insert(&mut self, vector: &[f32]) -> Result<u64, IndexError> {
        // Validate dimension
        if vector.len() != self.config.dimensions as usize {
            return Err(IndexError::DimensionMismatch {
                expected: self.config.dimensions as usize,
                actual: vector.len(),
            });
        }

        // Allocate ID
        let id = self.next_id;
        self.next_id += 1;

        // Store vector (append to contiguous storage)
        self.vectors.extend_from_slice(vector);

        // Update count and deleted bitmap
        self.count += 1;
        self.deleted.push(false);

        Ok(id)
    }

    /// Insert multiple vectors in batch.
    ///
    /// Returns the IDs assigned to each vector.
    ///
    /// # Errors
    ///
    /// Returns error if any vector has wrong dimension.
    pub fn insert_batch(&mut self, vectors: &[&[f32]]) -> Result<Vec<u64>, IndexError> {
        let mut ids = Vec::with_capacity(vectors.len());

        for vector in vectors {
            let id = self.insert(vector)?;
            ids.push(id);
        }

        Ok(ids)
    }

    /// Get a vector by ID.
    ///
    /// Returns None if the ID doesn't exist or was deleted.
    pub fn get(&self, id: u64) -> Option<&[f32]> {
        let idx = id as usize;

        // Check bounds
        if idx >= self.count as usize {
            return None;
        }

        // Check if deleted
        if self.deleted.get(idx).map(|b| *b).unwrap_or(true) {
            return None;
        }

        // Return vector slice
        let dim = self.config.dimensions as usize;
        let start = idx * dim;
        let end = start + dim;

        Some(&self.vectors[start..end])
    }

    /// Check if a vector ID exists and is not deleted.
    pub fn contains(&self, id: u64) -> bool {
        let idx = id as usize;
        idx < self.count as usize && !self.deleted.get(idx).map(|b| *b).unwrap_or(true)
    }
}
```

**Acceptance Criteria:**
- [ ] `insert()` validates dimensions
- [ ] `insert()` allocates sequential IDs
- [ ] `insert()` stores vectors correctly
- [ ] `insert_batch()` handles multiple vectors
- [ ] `get()` retrieves vectors by ID
- [ ] `contains()` checks existence
- [ ] Deleted vectors return None

**Deliverables:**
- Updated `src/index/flat.rs`

**Dependencies:** W40.1.1

**Estimated Duration:** 2 hours

**Agent:** RUST_ENGINEER

---

### W40.1.3: Unit Tests for Insertion

**Objective:** Create comprehensive unit tests for FlatIndex insertion.

**File:** `src/index/flat.rs` (tests module)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_flat_index() {
        let config = FlatIndexConfig::new(128);
        let index = FlatIndex::new(config);

        assert_eq!(index.dimensions(), 128);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_config_builder() {
        let config = FlatIndexConfig::new(64)
            .with_metric(Metric::DotProduct)
            .with_capacity(5000)
            .with_cleanup_threshold(0.3);

        assert_eq!(config.dimensions, 64);
        assert_eq!(config.metric, Metric::DotProduct);
        assert_eq!(config.initial_capacity, 5000);
        assert!((config.cleanup_threshold - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_insert_single() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();

        assert_eq!(id, 0);
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    #[test]
    fn test_insert_multiple() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let id1 = index.insert(&[1.0, 2.0, 3.0]).unwrap();
        let id2 = index.insert(&[4.0, 5.0, 6.0]).unwrap();
        let id3 = index.insert(&[7.0, 8.0, 9.0]).unwrap();

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn test_insert_dimension_mismatch() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let result = index.insert(&[1.0, 2.0]); // Wrong dimension

        assert!(result.is_err());
        assert!(matches!(result, Err(IndexError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_get_vector() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        index.insert(&[1.0, 2.0, 3.0]).unwrap();
        index.insert(&[4.0, 5.0, 6.0]).unwrap();

        let v0 = index.get(0).unwrap();
        let v1 = index.get(1).unwrap();

        assert_eq!(v0, &[1.0, 2.0, 3.0]);
        assert_eq!(v1, &[4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_get_nonexistent() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        index.insert(&[1.0, 2.0, 3.0]).unwrap();

        assert!(index.get(99).is_none());
    }

    #[test]
    fn test_contains() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        index.insert(&[1.0, 2.0, 3.0]).unwrap();

        assert!(index.contains(0));
        assert!(!index.contains(1));
        assert!(!index.contains(99));
    }

    #[test]
    fn test_insert_batch() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let vectors: Vec<&[f32]> = vec![
            &[1.0, 2.0, 3.0],
            &[4.0, 5.0, 6.0],
            &[7.0, 8.0, 9.0],
        ];

        let ids = index.insert_batch(&vectors).unwrap();

        assert_eq!(ids, vec![0, 1, 2]);
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn test_capacity_growth() {
        let config = FlatIndexConfig::new(3).with_capacity(2);
        let mut index = FlatIndex::new(config);

        // Insert more than initial capacity
        for i in 0..10 {
            index.insert(&[i as f32; 3]).unwrap();
        }

        assert_eq!(index.len(), 10);

        // All vectors retrievable
        for i in 0..10 {
            assert!(index.contains(i as u64));
        }
    }
}
```

**Acceptance Criteria:**
- [ ] `test_new_flat_index` passes
- [ ] `test_config_builder` passes
- [ ] `test_insert_single` passes
- [ ] `test_insert_multiple` passes
- [ ] `test_insert_dimension_mismatch` passes
- [ ] `test_get_vector` passes
- [ ] `test_get_nonexistent` passes
- [ ] `test_contains` passes
- [ ] `test_insert_batch` passes
- [ ] `test_capacity_growth` passes
- [ ] All tests pass with `cargo test`

**Deliverables:**
- Test module in `src/index/flat.rs`

**Dependencies:** W40.1.2

**Estimated Duration:** 1.5 hours

**Agent:** TEST_ENGINEER

---

## Verification Strategy

### Unit Tests
- `test_new_flat_index`: Verify default construction
- `test_config_builder`: Verify builder pattern
- `test_insert_*`: Verify insertion behavior
- `test_get_*`: Verify retrieval behavior
- `test_contains`: Verify existence checks

### Compile Checks
- `cargo build` succeeds
- `cargo clippy -- -D warnings` passes
- Doc comments render correctly

---

## Exit Criteria for Day 1

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| FlatIndex struct defined | Compiles | [ ] |
| FlatIndexConfig with builder | Tests pass | [ ] |
| insert() works correctly | `test_insert_*` pass | [ ] |
| get() retrieves vectors | `test_get_*` pass | [ ] |
| contains() checks existence | `test_contains` passes | [ ] |
| 10+ unit tests passing | `cargo test` | [ ] |
| Clippy clean | 0 warnings | [ ] |

---

## Technical Notes

### Memory Layout Decision

**Column-major vs Row-major:**

We use **row-major** (contiguous per-vector) for simplicity:
```
vectors = [v0_d0, v0_d1, ..., v0_dn, v1_d0, v1_d1, ..., v1_dn, ...]
```

This allows:
- Simple slicing: `&vectors[id*dim..(id+1)*dim]`
- Cache-friendly for single-vector access
- Straightforward append

Column-major would require restructuring for SIMD batch operations but adds complexity. Can optimize in Day 3 if needed.

### ID Allocation

Sequential IDs (0, 1, 2, ...) are simplest:
- O(1) lookup via array indexing
- No hash map overhead
- Matches HNSW VectorId pattern

Deleted IDs are tracked in BitVec, not reused (to preserve ID stability).

---

**Day 1 Total:** 5 hours
**Agent:** RUST_ENGINEER
**Status:** [DRAFT]

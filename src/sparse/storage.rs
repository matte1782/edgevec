//! Sparse vector storage using packed arrays.
//!
//! This module provides persistent storage for sparse vectors with
//! efficient memory layout and serialization support.
//!
//! # Memory Layout
//!
//! All vectors' indices and values are concatenated into packed arrays.
//! The `offsets` array tracks where each vector starts:
//!
//! ```text
//! Vector 0: indices[0..offsets[1]], values[0..offsets[1]]
//! Vector 1: indices[offsets[1]..offsets[2]], values[offsets[1]..offsets[2]]
//! ...
//! ```
//!
//! # Performance Targets (RFC-007)
//!
//! - Insert: P50 <50us, P99 <100us
//! - Get: <1us
//! - Iteration: <100ms for 100k vectors
//!
//! # Example
//!
//! ```rust
//! use edgevec::sparse::{SparseStorage, SparseVector};
//!
//! let mut storage = SparseStorage::new();
//! let vector = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100)?;
//! let id = storage.insert(&vector)?;
//!
//! let retrieved = storage.get(id).unwrap();
//! assert_eq!(retrieved.indices(), vector.indices());
//! # Ok::<(), edgevec::sparse::SparseError>(())
//! ```

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use bitvec::prelude::*;
use serde::{Deserialize, Serialize};

use crate::sparse::{SparseError, SparseVector};

// =============================================================================
// BINARY FORMAT CONSTANTS
// =============================================================================

/// Magic number for EdgeVec Sparse Vector format: "ESPV" in ASCII.
///
/// Used to identify valid sparse storage files.
pub const SPARSE_MAGIC: [u8; 4] = [b'E', b'S', b'P', b'V'];

/// Current format version. Increment on breaking changes.
///
/// Version history:
/// - v1: Initial format with CSR-like packed arrays
/// - v2: Added CRC32 checksum as final 4 bytes after payload
pub const SPARSE_FORMAT_VERSION: u32 = 2;

/// Maximum supported format version for backward-compatible loading.
/// Version 1 files are loaded without checksum validation.
const SPARSE_MAX_SUPPORTED_VERSION: u32 = 2;

// =============================================================================
// SPARSE ID
// =============================================================================

/// Unique identifier for sparse vectors.
///
/// Uses u64 for compatibility with dense VectorId.
/// IDs are assigned monotonically and never reused.
///
/// # Example
///
/// ```rust
/// use edgevec::sparse::SparseId;
///
/// let id = SparseId::new(42);
/// assert_eq!(id.as_u64(), 42);
///
/// let id2: SparseId = 100u64.into();
/// assert_eq!(u64::from(id2), 100);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SparseId(u64);

impl SparseId {
    /// Create a new SparseId from a u64.
    #[inline]
    #[must_use]
    pub const fn new(id: u64) -> Self {
        SparseId(id)
    }

    /// Get the underlying u64 value.
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for SparseId {
    #[inline]
    fn from(id: u64) -> Self {
        SparseId(id)
    }
}

impl From<SparseId> for u64 {
    #[inline]
    fn from(id: SparseId) -> Self {
        id.0
    }
}

impl std::fmt::Display for SparseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SparseId({})", self.0)
    }
}

// =============================================================================
// SPARSE STORAGE
// =============================================================================

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
/// NOT thread-safe. Wrap in `Arc<RwLock<>>` for concurrent access.
///
/// # Memory Estimate (100k vectors, avg 50 non-zero)
///
/// ```text
/// indices: 100k * 50 * 4 bytes = 20 MB
/// values:  100k * 50 * 4 bytes = 20 MB
/// offsets: 100k * 4 bytes      = 0.4 MB
/// dims:    100k * 4 bytes      = 0.4 MB
/// deleted: 100k / 8 bytes      = 12.5 KB
/// Total: ~41 MB
/// ```
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

impl SparseStorage {
    /// Create empty storage.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseStorage;
    ///
    /// let storage = SparseStorage::new();
    /// assert!(storage.is_empty());
    /// ```
    #[must_use]
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
    #[must_use]
    pub fn with_capacity(num_vectors: usize, avg_nnz: usize) -> Self {
        let total_elements = num_vectors.saturating_mul(avg_nnz);
        Self {
            indices: Vec::with_capacity(total_elements),
            values: Vec::with_capacity(total_elements),
            offsets: {
                let mut v = Vec::with_capacity(num_vectors.saturating_add(1));
                v.push(0);
                v
            },
            dims: Vec::with_capacity(num_vectors),
            deleted: BitVec::with_capacity(num_vectors),
            next_id: 0,
        }
    }

    /// Returns the number of vectors (including deleted).
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// assert_eq!(storage.len(), 0);
    ///
    /// storage.insert(&SparseVector::singleton(0, 1.0, 100)?)?;
    /// assert_eq!(storage.len(), 1);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.dims.len()
    }

    /// Returns true if no vectors are stored.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dims.is_empty()
    }

    /// Returns the number of non-deleted vectors.
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
    /// assert_eq!(storage.live_count(), 2);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn live_count(&self) -> usize {
        self.deleted.count_zeros()
    }

    /// Returns the number of deleted vectors.
    ///
    /// Useful for:
    /// - Monitoring storage fragmentation
    /// - Deciding when to run compaction
    /// - Debugging and testing
    ///
    /// # Complexity
    ///
    /// O(n/64) - BitVec popcount is efficient.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// let id1 = storage.insert(&v)?;
    /// let id2 = storage.insert(&v)?;
    ///
    /// assert_eq!(storage.deleted_count(), 0);
    ///
    /// storage.delete(id1)?;
    /// assert_eq!(storage.deleted_count(), 1);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn deleted_count(&self) -> usize {
        self.deleted.count_ones()
    }

    /// Returns the number of active (non-deleted) vectors.
    ///
    /// Alias for `live_count()` for consistency with API naming.
    ///
    /// # Formula
    ///
    /// `active_count() = total_count() - deleted_count()`
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// storage.insert(&v)?;
    /// storage.insert(&v)?;
    ///
    /// assert_eq!(storage.active_count(), 2);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.live_count()
    }

    /// Returns the total number of vectors (including deleted).
    ///
    /// Alias for `len()` for consistency with API naming.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// let id = storage.insert(&v)?;
    /// storage.insert(&v)?;
    ///
    /// storage.delete(id)?;
    ///
    /// assert_eq!(storage.total_count(), 2);  // Still 2 (soft delete)
    /// assert_eq!(storage.active_count(), 1); // Only 1 active
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.len()
    }

    /// Returns the deletion ratio (deleted / total).
    ///
    /// Useful for deciding when to compact:
    /// - Ratio > 0.5: Consider compaction
    /// - Ratio > 0.7: Recommend compaction
    ///
    /// # Returns
    ///
    /// Ratio in [0.0, 1.0], or 0.0 if storage is empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    ///
    /// // Empty storage
    /// assert_eq!(storage.deletion_ratio(), 0.0);
    ///
    /// // Insert 4 vectors
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// for _ in 0..4 {
    ///     storage.insert(&v)?;
    /// }
    ///
    /// // Delete 2 of 4 = 50%
    /// use edgevec::sparse::SparseId;
    /// storage.delete(SparseId::new(0))?;
    /// storage.delete(SparseId::new(1))?;
    ///
    /// assert!((storage.deletion_ratio() - 0.5).abs() < 0.01);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn deletion_ratio(&self) -> f32 {
        if self.dims.is_empty() {
            return 0.0;
        }
        self.deleted_count() as f32 / self.dims.len() as f32
    }

    /// Returns total memory used in bytes (approximate).
    ///
    /// Includes:
    /// - Packed indices array
    /// - Packed values array
    /// - Offsets array
    /// - Dims array
    /// - Deletion bitmap
    ///
    /// Does not include struct overhead or allocator metadata.
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        let indices_bytes = self.indices.capacity() * std::mem::size_of::<u32>();
        let values_bytes = self.values.capacity() * std::mem::size_of::<f32>();
        let offsets_bytes = self.offsets.capacity() * std::mem::size_of::<u32>();
        let dims_bytes = self.dims.capacity() * std::mem::size_of::<u32>();
        let deleted_bytes = (self.deleted.capacity() + 7) / 8;

        indices_bytes + values_bytes + offsets_bytes + dims_bytes + deleted_bytes
    }

    /// Returns the total number of non-zero elements across all vectors.
    #[inline]
    #[must_use]
    pub fn total_nnz(&self) -> usize {
        self.indices.len()
    }

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
    /// # Errors
    ///
    /// Returns [`SparseError::EmptyVector`] if nnz == 0.
    #[cfg(test)]
    fn validate_for_insert(vector: &SparseVector) -> Result<(), SparseError> {
        // Check nnz > 0 (defensive - SparseVector already guarantees this)
        if vector.nnz() == 0 {
            return Err(SparseError::EmptyVector);
        }

        // NOTE: We intentionally do NOT enforce dimension consistency.
        // Different vectors can have different dimensions (vocabulary sizes).
        // This is a deliberate design choice per RFC-007 discussion.

        Ok(())
    }

    // =========================================================================
    // TEST ACCESSORS (cfg(test) gated)
    // =========================================================================

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
        &self.offsets[..self.offsets.len().saturating_sub(1)]
    }

    /// Get slice of dimensions (for testing).
    #[cfg(test)]
    pub(crate) fn dims_slice(&self) -> &[u32] {
        &self.dims
    }

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
    /// # Errors
    ///
    /// This function is infallible for valid `SparseVector` inputs.
    /// The `Result` type is for API consistency with batch operations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let vector = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100)?;
    /// let id = storage.insert(&vector)?;
    /// assert_eq!(id.as_u64(), 0);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn insert(&mut self, vector: &SparseVector) -> Result<SparseId, SparseError> {
        // Assign ID (with overflow protection)
        let id = SparseId::new(self.next_id);
        self.next_id = self.next_id.checked_add(1).ok_or(SparseError::IdOverflow)?;

        // Append indices
        self.indices.extend_from_slice(vector.indices());

        // Append values
        self.values.extend_from_slice(vector.values());

        // Record new offset (end of this vector = start of next)
        // Intentional truncation: storage is designed for <4B elements
        #[allow(clippy::cast_possible_truncation)]
        let new_offset = self.indices.len() as u32;
        self.offsets.push(new_offset);

        // Record dimension
        self.dims.push(vector.dim());

        // Mark as not deleted
        self.deleted.push(false);

        Ok(id)
    }

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
    /// # Errors
    ///
    /// This function is infallible for valid `SparseVector` inputs.
    /// The `Result` type is for API consistency.
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
        let total_nnz: usize = vectors.iter().map(SparseVector::nnz).sum();

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
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn get(&self, id: SparseId) -> Option<SparseVector> {
        // Intentional: u64 to usize is safe on 64-bit, on 32-bit we just bounds-check
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
    #[allow(clippy::cast_possible_truncation)]
    pub fn get_indices(&self, id: SparseId) -> Option<&[u32]> {
        let idx = id.as_u64() as usize;

        // Bounds check
        if idx >= self.dims.len() {
            return None;
        }

        // Deletion check
        if self.deleted[idx] {
            return None;
        }

        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;

        Some(&self.indices[start..end])
    }

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
    #[allow(clippy::cast_possible_truncation)]
    pub fn get_values(&self, id: SparseId) -> Option<&[f32]> {
        let idx = id.as_u64() as usize;

        // Bounds check
        if idx >= self.dims.len() {
            return None;
        }

        // Deletion check
        if self.deleted[idx] {
            return None;
        }

        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;

        Some(&self.values[start..end])
    }

    /// Get dimension for a sparse vector.
    ///
    /// Returns `None` if:
    /// - ID is out of bounds
    /// - Vector has been deleted
    ///
    /// # Complexity
    ///
    /// - Time: O(1)
    /// - Space: O(1)
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let vec = SparseVector::new(vec![0, 5], vec![0.1, 0.2], 100)?;
    /// let id = storage.insert(&vec)?;
    ///
    /// assert_eq!(storage.get_dim(id), Some(100));
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    pub fn get_dim(&self, id: SparseId) -> Option<u32> {
        let idx = id.as_u64() as usize;

        // Bounds check
        if idx >= self.dims.len() {
            return None;
        }

        // Deletion check
        if self.deleted[idx] {
            return None;
        }

        Some(self.dims[idx])
    }

    /// Check if an ID exists and is not deleted.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector, SparseId};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// let id = storage.insert(&v)?;
    ///
    /// assert!(storage.contains(id));
    /// assert!(!storage.contains(SparseId::new(999)));
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[inline]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn contains(&self, id: SparseId) -> bool {
        let idx = id.as_u64() as usize;
        idx < self.dims.len() && !self.deleted[idx]
    }

    /// Check if a sparse vector is deleted or non-existent.
    ///
    /// # Returns
    ///
    /// - `true` if vector is deleted OR ID does not exist
    /// - `false` if vector exists and is not deleted
    ///
    /// # Note
    ///
    /// Returns `true` for non-existent IDs to simplify caller logic.
    /// Use `exists()` if you need to distinguish between deleted and non-existent.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector, SparseId};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// let id = storage.insert(&v)?;
    ///
    /// assert!(!storage.is_deleted(id));  // Active vector
    ///
    /// storage.delete(id)?;
    /// assert!(storage.is_deleted(id));   // Deleted vector
    ///
    /// assert!(storage.is_deleted(SparseId::new(999)));  // Non-existent
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[inline]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn is_deleted(&self, id: SparseId) -> bool {
        let idx = id.as_u64() as usize;
        // Out of bounds = treat as deleted (doesn't exist)
        if idx >= self.dims.len() {
            return true;
        }
        self.deleted[idx]
    }

    /// Check if a sparse vector ID exists (regardless of deletion status).
    ///
    /// # Returns
    ///
    /// - `true` if ID is within valid range
    /// - `false` if ID is out of range
    ///
    /// # Use Case
    ///
    /// Distinguish between deleted and non-existent IDs:
    /// - `exists() == false`: ID was never assigned
    /// - `exists() == true && is_deleted() == true`: ID was deleted
    /// - `exists() == true && is_deleted() == false`: ID is active
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector, SparseId};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// let id = storage.insert(&v)?;
    ///
    /// // Active vector
    /// assert!(storage.exists(id));
    /// assert!(!storage.is_deleted(id));
    ///
    /// // Deleted vector
    /// storage.delete(id)?;
    /// assert!(storage.exists(id));        // Still exists
    /// assert!(storage.is_deleted(id));    // But marked deleted
    ///
    /// // Non-existent
    /// let fake_id = SparseId::new(999);
    /// assert!(!storage.exists(fake_id));
    /// assert!(storage.is_deleted(fake_id));  // is_deleted returns true
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[inline]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn exists(&self, id: SparseId) -> bool {
        let idx = id.as_u64() as usize;
        idx < self.dims.len()
    }

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
    /// # Errors
    ///
    /// Returns [`SparseError::IdNotFound`] if the ID doesn't exist.
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
    #[allow(clippy::cast_possible_truncation)]
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
    ///
    /// # Errors
    ///
    /// Returns [`SparseError::IdNotFound`] if the ID doesn't exist.
    #[allow(clippy::cast_possible_truncation)]
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
    /// Returns [`SparseError::IdNotFound`] if any ID is not found.
    /// Deletion is atomic: if any ID is invalid, no deletions are performed.
    #[allow(clippy::cast_possible_truncation)]
    pub fn delete_batch(&mut self, ids: &[SparseId]) -> Result<usize, SparseError> {
        // Validate all IDs first
        for id in ids {
            let vector_idx = id.as_u64() as usize;
            if vector_idx >= self.dims.len() {
                return Err(SparseError::IdNotFound(id.as_u64()));
            }
        }

        // Perform deletions
        let mut deleted_count = 0;
        for id in ids {
            let vector_idx = id.as_u64() as usize;
            if !self.deleted[vector_idx] {
                self.deleted.set(vector_idx, true);
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }

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
    ///     println!("ID {}: nnz={}", id, vector.nnz());
    /// }
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn iter(&self) -> SparseStorageIter<'_> {
        SparseStorageIter {
            storage: self,
            current_idx: 0,
        }
    }

    /// Iterate over all non-deleted IDs (without reconstructing vectors).
    ///
    /// More efficient than `iter()` when you only need IDs.
    #[allow(clippy::cast_possible_truncation)]
    pub fn ids(&self) -> impl Iterator<Item = SparseId> + '_ {
        (0..self.dims.len())
            .filter(move |&idx| !self.deleted[idx])
            .map(|idx| SparseId::new(idx as u64))
    }

    // =========================================================================
    // SERIALIZATION
    // =========================================================================

    /// Save storage to a binary file.
    ///
    /// # Binary Format
    ///
    /// The file starts with a header for validation:
    /// - Magic number (4 bytes): "ESPV"
    /// - Version (4 bytes): u32 little-endian
    ///
    /// Followed by the data:
    /// - Count (8 bytes): u64 LE, number of vectors
    /// - Offsets (count+1 * 4 bytes): u32 LE per offset
    /// - Dims (count * 4 bytes): u32 LE per vector
    /// - Deleted ((count + 7) / 8 bytes): packed bits
    /// - Next ID (8 bytes): u64 LE
    /// - Total NNZ (8 bytes): u64 LE
    /// - Indices (total_nnz * 4 bytes): u32 LE
    /// - Values (total_nnz * 4 bytes): f32 LE
    ///
    /// # Errors
    ///
    /// Returns `SparseError::Io` if the file cannot be written.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use edgevec::sparse::SparseStorage;
    /// use std::path::Path;
    ///
    /// let storage = SparseStorage::new();
    /// storage.save(Path::new("sparse.espv"))?;
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    pub fn save(&self, path: &Path) -> Result<(), SparseError> {
        let file = File::create(path).map_err(|e| SparseError::Io(e.to_string()))?;
        let mut writer = BufWriter::new(file);

        // Serialize to bytes (includes header, payload, and CRC32 checksum)
        let bytes = self.to_bytes();
        writer
            .write_all(&bytes)
            .map_err(|e| SparseError::Io(e.to_string()))?;

        writer.flush().map_err(|e| SparseError::Io(e.to_string()))?;
        Ok(())
    }

    /// Serialize storage to bytes (in-memory).
    ///
    /// Returns a byte vector containing the complete serialized storage.
    /// Useful for benchmarking, caching, or network transmission.
    ///
    /// # Format
    ///
    /// Same format as `save()`:
    /// - Magic number "ESPV" (4 bytes)
    /// - Version (4 bytes, little-endian u32)
    /// - Count (8 bytes, little-endian u64)
    /// - Offsets ((count+1) * 4 bytes)
    /// - Dims (count * 4 bytes)
    /// - Deleted bitmap (ceil(count/8) bytes)
    /// - next_id (8 bytes)
    /// - total_nnz (8 bytes)
    /// - Indices (total_nnz * 4 bytes)
    /// - Values (total_nnz * 4 bytes)
    /// - CRC32 checksum (4 bytes, v2+) — covers everything after the 8-byte header
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// storage.insert(&v)?;
    ///
    /// let bytes = storage.to_bytes();
    /// let restored = SparseStorage::from_bytes(&bytes)?;
    ///
    /// assert_eq!(restored.len(), 1);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_bytes(&self) -> Vec<u8> {
        let count = self.dims.len();
        let total_nnz = self.indices.len();
        let deleted_bytes_len = (count + 7) / 8;

        // Pre-allocate with exact size (including CRC32 checksum for v2)
        let capacity = 4 // magic
            + 4 // version
            + 8 // count
            + (count + 1) * 4 // offsets
            + count * 4 // dims
            + deleted_bytes_len // deleted bitmap
            + 8 // next_id
            + 8 // total_nnz
            + total_nnz * 4 // indices
            + total_nnz * 4 // values
            + 4; // CRC32 checksum

        let mut bytes = Vec::with_capacity(capacity);

        // Header
        bytes.extend_from_slice(&SPARSE_MAGIC);
        bytes.extend_from_slice(&SPARSE_FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&(count as u64).to_le_bytes());

        // Offsets
        for offset in &self.offsets {
            bytes.extend_from_slice(&offset.to_le_bytes());
        }

        // Dims
        for dim in &self.dims {
            bytes.extend_from_slice(&dim.to_le_bytes());
        }

        // Deleted bitmap
        bytes.extend_from_slice(&self.pack_deleted_bits());

        // next_id
        bytes.extend_from_slice(&self.next_id.to_le_bytes());

        // total_nnz
        bytes.extend_from_slice(&(total_nnz as u64).to_le_bytes());

        // Indices
        for idx in &self.indices {
            bytes.extend_from_slice(&idx.to_le_bytes());
        }

        // Values
        for val in &self.values {
            bytes.extend_from_slice(&val.to_le_bytes());
        }

        // CRC32 checksum covers everything after the 8-byte header (magic + version).
        // The header itself is excluded so that version detection remains independent
        // of checksum validation.
        let payload = &bytes[8..]; // skip magic (4) + version (4)
        let checksum = crc32fast::hash(payload);
        bytes.extend_from_slice(&checksum.to_le_bytes());

        bytes
    }

    /// Deserialize storage from bytes (in-memory).
    ///
    /// # Errors
    ///
    /// - `SparseError::InvalidMagic` if magic number doesn't match
    /// - `SparseError::UnsupportedVersion` if version is not supported
    /// - `SparseError::CorruptedData` if data is truncated or invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v = SparseVector::singleton(0, 1.0, 100)?;
    /// storage.insert(&v)?;
    ///
    /// let bytes = storage.to_bytes();
    /// let restored = SparseStorage::from_bytes(&bytes)?;
    ///
    /// assert_eq!(restored.len(), storage.len());
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SparseError> {
        use std::io::{Cursor, Read};

        let mut cursor = Cursor::new(bytes);

        // Read and validate magic
        let mut magic = [0u8; 4];
        cursor
            .read_exact(&mut magic)
            .map_err(|e| SparseError::CorruptedData(format!("Failed to read magic: {e}")))?;
        if magic != SPARSE_MAGIC {
            return Err(SparseError::InvalidMagic {
                expected: SPARSE_MAGIC,
                found: magic,
            });
        }

        // Read and validate version (accept v1 for backward compat, v2 for current)
        let mut version_bytes = [0u8; 4];
        cursor
            .read_exact(&mut version_bytes)
            .map_err(|e| SparseError::CorruptedData(format!("Failed to read version: {e}")))?;
        let version = u32::from_le_bytes(version_bytes);
        if version == 0 || version > SPARSE_MAX_SUPPORTED_VERSION {
            return Err(SparseError::UnsupportedVersion {
                expected: SPARSE_FORMAT_VERSION,
                found: version,
            });
        }

        // For v2+, validate CRC32 checksum before parsing payload.
        // The checksum covers bytes[8..len-4] (everything after header, before checksum).
        if version >= 2 {
            if bytes.len() < 12 {
                return Err(SparseError::CorruptedData(
                    "File too short for v2 format (missing checksum)".to_string(),
                ));
            }
            let payload = &bytes[8..bytes.len() - 4];
            let expected_checksum = u32::from_le_bytes([
                bytes[bytes.len() - 4],
                bytes[bytes.len() - 3],
                bytes[bytes.len() - 2],
                bytes[bytes.len() - 1],
            ]);
            let actual_checksum = crc32fast::hash(payload);
            if expected_checksum != actual_checksum {
                return Err(SparseError::CorruptedData(format!(
                    "CRC32 checksum mismatch: expected {expected_checksum:#010X}, got {actual_checksum:#010X}"
                )));
            }
        }

        // Read count
        let mut count_bytes = [0u8; 8];
        cursor
            .read_exact(&mut count_bytes)
            .map_err(|e| SparseError::CorruptedData(format!("Failed to read count: {e}")))?;
        let count = u64::from_le_bytes(count_bytes) as usize;

        // Read offsets
        let mut offsets = Vec::with_capacity(count + 1);
        for _ in 0..=count {
            let mut buf = [0u8; 4];
            cursor
                .read_exact(&mut buf)
                .map_err(|e| SparseError::CorruptedData(format!("Failed to read offset: {e}")))?;
            offsets.push(u32::from_le_bytes(buf));
        }

        // Read dims
        let mut dims = Vec::with_capacity(count);
        for _ in 0..count {
            let mut buf = [0u8; 4];
            cursor
                .read_exact(&mut buf)
                .map_err(|e| SparseError::CorruptedData(format!("Failed to read dim: {e}")))?;
            dims.push(u32::from_le_bytes(buf));
        }

        // Read deleted bitmap
        let deleted_bytes_len = (count + 7) / 8;
        let mut deleted_bytes = vec![0u8; deleted_bytes_len];
        cursor.read_exact(&mut deleted_bytes).map_err(|e| {
            SparseError::CorruptedData(format!("Failed to read deleted bitmap: {e}"))
        })?;

        let deleted = Self::unpack_deleted_bits(&deleted_bytes, count);

        // Read next_id
        let mut next_id_bytes = [0u8; 8];
        cursor
            .read_exact(&mut next_id_bytes)
            .map_err(|e| SparseError::CorruptedData(format!("Failed to read next_id: {e}")))?;
        let next_id = u64::from_le_bytes(next_id_bytes);

        // Read total_nnz
        let mut nnz_bytes = [0u8; 8];
        cursor
            .read_exact(&mut nnz_bytes)
            .map_err(|e| SparseError::CorruptedData(format!("Failed to read total_nnz: {e}")))?;
        let total_nnz = u64::from_le_bytes(nnz_bytes) as usize;

        // Read indices
        let mut indices = Vec::with_capacity(total_nnz);
        for _ in 0..total_nnz {
            let mut buf = [0u8; 4];
            cursor
                .read_exact(&mut buf)
                .map_err(|e| SparseError::CorruptedData(format!("Failed to read index: {e}")))?;
            indices.push(u32::from_le_bytes(buf));
        }

        // Read values
        let mut values = Vec::with_capacity(total_nnz);
        for _ in 0..total_nnz {
            let mut buf = [0u8; 4];
            cursor
                .read_exact(&mut buf)
                .map_err(|e| SparseError::CorruptedData(format!("Failed to read value: {e}")))?;
            values.push(f32::from_le_bytes(buf));
        }

        Ok(Self {
            indices,
            values,
            offsets,
            dims,
            deleted,
            next_id,
        })
    }

    /// Load storage from a binary file.
    ///
    /// # Format Validation
    ///
    /// - Validates magic number is "ESPV"
    /// - Validates version is compatible (currently only v1)
    ///
    /// # Errors
    ///
    /// - `SparseError::InvalidMagic` if magic number doesn't match
    /// - `SparseError::UnsupportedVersion` if version is not supported
    /// - `SparseError::Io` if file cannot be read
    /// - `SparseError::CorruptedData` if data is inconsistent
    ///
    /// # Example
    ///
    /// ```no_run
    /// use edgevec::sparse::SparseStorage;
    /// use std::path::Path;
    ///
    /// let storage = SparseStorage::load(Path::new("sparse.espv"))?;
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    pub fn load(path: &Path) -> Result<Self, SparseError> {
        // Read entire file into memory, then delegate to from_bytes() which
        // handles version detection, CRC32 validation, and parsing.
        let mut file = File::open(path).map_err(|e| SparseError::Io(e.to_string()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| SparseError::Io(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Get the next ID that will be assigned.
    ///
    /// Useful for verification after load.
    #[must_use]
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Pack deleted bits into bytes.
    fn pack_deleted_bits(&self) -> Vec<u8> {
        let count = self.deleted.len();
        let byte_count = (count + 7) / 8;
        let mut bytes = vec![0u8; byte_count];

        for (i, b) in self.deleted.iter().enumerate() {
            if *b {
                bytes[i / 8] |= 1 << (i % 8);
            }
        }

        bytes
    }

    /// Unpack deleted bits from bytes.
    fn unpack_deleted_bits(bytes: &[u8], count: usize) -> BitVec {
        let mut bits = BitVec::with_capacity(count);
        for i in 0..count {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            let is_deleted = if byte_idx < bytes.len() {
                (bytes[byte_idx] >> bit_idx) & 1 == 1
            } else {
                false
            };
            bits.push(is_deleted);
        }
        bits
    }
}

impl Default for SparseStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a SparseStorage {
    type Item = (SparseId, SparseVector);
    type IntoIter = SparseStorageIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// =============================================================================
// ITERATOR
// =============================================================================

/// Iterator over non-deleted sparse vectors.
pub struct SparseStorageIter<'a> {
    storage: &'a SparseStorage,
    current_idx: usize,
}

impl Iterator for SparseStorageIter<'_> {
    type Item = (SparseId, SparseVector);

    #[allow(clippy::cast_possible_truncation)]
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

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
#[allow(
    clippy::uninlined_format_args,
    clippy::default_trait_access,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::redundant_closure_for_method_calls,
    clippy::range_plus_one,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    // ============= SparseId Tests =============

    #[test]
    fn test_sparse_id_new() {
        let id = SparseId::new(42);
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn test_sparse_id_from_u64() {
        let id: SparseId = 100u64.into();
        assert_eq!(id.as_u64(), 100);
    }

    #[test]
    fn test_sparse_id_into_u64() {
        let id = SparseId::new(50);
        let val: u64 = id.into();
        assert_eq!(val, 50);
    }

    #[test]
    fn test_sparse_id_display() {
        let id = SparseId::new(123);
        assert_eq!(format!("{}", id), "SparseId(123)");
    }

    #[test]
    fn test_sparse_id_copy_clone() {
        let id1 = SparseId::new(42);
        let id2 = id1; // Copy
        let id3 = id1; // Copy (same as clone for Copy types)
        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_sparse_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SparseId::new(1));
        set.insert(SparseId::new(2));
        set.insert(SparseId::new(1)); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // ============= SparseStorage Basic Tests =============

    #[test]
    fn test_storage_new() {
        let storage = SparseStorage::new();
        assert!(storage.is_empty());
        assert_eq!(storage.len(), 0);
        assert_eq!(storage.live_count(), 0);
    }

    #[test]
    fn test_storage_with_capacity() {
        let storage = SparseStorage::with_capacity(1000, 50);
        assert!(storage.is_empty());
        // Verify capacity was allocated
        assert!(storage.memory_usage() > 0);
    }

    #[test]
    fn test_storage_default() {
        let storage: SparseStorage = Default::default();
        assert!(storage.is_empty());
    }

    // ============= Insert Tests =============

    #[test]
    fn test_insert_single() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();

        let id = storage.insert(&v).unwrap();

        assert_eq!(id.as_u64(), 0);
        assert_eq!(storage.len(), 1);
        assert_eq!(storage.live_count(), 1);
        assert_eq!(storage.total_nnz(), 3);
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

    // ============= Get Tests =============

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

    // ============= Delete Tests =============

    #[test]
    fn test_delete_valid() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        let id = storage.insert(&v).unwrap();

        let result = storage.delete(id).unwrap();
        assert!(result); // Was deleted
        assert!(storage.is_deleted(id));
        assert!(!storage.contains(id));
        assert_eq!(storage.live_count(), 0);
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

    #[test]
    fn test_delete_batch_success() {
        let mut storage = SparseStorage::new();

        for i in 0..5 {
            let v = SparseVector::singleton(i as u32, 1.0, 100).unwrap();
            storage.insert(&v).unwrap();
        }

        let deleted = storage
            .delete_batch(&[SparseId::new(0), SparseId::new(2), SparseId::new(4)])
            .unwrap();

        assert_eq!(deleted, 3);
        assert_eq!(storage.live_count(), 2);
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

    // ============= Iterator Tests =============

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

    // ============= Roundtrip Tests =============

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

        let ids: Vec<_> = vectors.iter().map(|v| storage.insert(v).unwrap()).collect();

        // Verify all roundtrip correctly
        for (id, original) in ids.iter().zip(vectors.iter()) {
            let retrieved = storage.get(*id).unwrap();
            assert_eq!(retrieved.indices(), original.indices());
            assert_eq!(retrieved.values(), original.values());
            assert_eq!(retrieved.dim(), original.dim());
        }
    }

    // ============= Memory Usage Tests =============

    #[test]
    fn test_memory_usage_grows() {
        let mut storage = SparseStorage::new();
        let initial = storage.memory_usage();

        // Insert some vectors
        for _ in 0..100 {
            let v = SparseVector::new(vec![0, 1, 2], vec![1.0, 2.0, 3.0], 100).unwrap();
            storage.insert(&v).unwrap();
        }

        assert!(storage.memory_usage() > initial);
    }

    // ============= Day 2: Insert Packed Array Tests =============

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
        let v2 = SparseVector::new(vec![0], vec![1.0], 500).unwrap();
        let v3 = SparseVector::new(vec![0], vec![1.0], 10000).unwrap();

        storage.insert(&v1).unwrap();
        storage.insert(&v2).unwrap();
        storage.insert(&v3).unwrap();

        assert_eq!(storage.dims_slice(), &[100, 500, 10000]);
    }

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

    #[test]
    fn test_insert_id_monotonic() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::singleton(0, 1.0, 10).unwrap();

        let mut prev_id = storage.insert(&vector).unwrap();
        for _ in 0..100 {
            let current_id = storage.insert(&vector).unwrap();
            assert!(current_id.as_u64() > prev_id.as_u64());
            prev_id = current_id;
        }
    }

    #[test]
    fn test_insert_id_starts_from_zero() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::singleton(0, 1.0, 10).unwrap();

        let first_id = storage.insert(&vector).unwrap();
        assert_eq!(first_id.as_u64(), 0);
    }

    #[test]
    fn test_insert_many_vectors() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::new(vec![0, 1, 2], vec![0.1, 0.2, 0.3], 100).unwrap();

        // Insert 1000 vectors
        for i in 0..1000 {
            let id = storage.insert(&vector).unwrap();
            assert_eq!(id.as_u64(), i);
        }

        assert_eq!(storage.len(), 1000);
        // 1000 vectors * 3 elements each = 3000 total elements
        assert_eq!(storage.indices_slice().len(), 3000);
        assert_eq!(storage.values_slice().len(), 3000);
    }

    #[test]
    fn test_insert_singleton() {
        let mut storage = SparseStorage::new();
        let vector = SparseVector::singleton(42, 0.5, 100).unwrap();

        storage.insert(&vector).unwrap();

        assert_eq!(storage.len(), 1);
        assert_eq!(storage.indices_slice(), &[42]);
        assert_eq!(storage.values_slice(), &[0.5]);
        assert_eq!(storage.offsets_slice(), &[0]);
    }

    // ============= Day 2: Validation Tests =============

    #[test]
    fn test_validate_for_insert_accepts_valid() {
        let vector = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();

        assert!(SparseStorage::validate_for_insert(&vector).is_ok());
    }

    #[test]
    fn test_validate_for_insert_accepts_singleton() {
        let vector = SparseVector::singleton(0, 1.0, 100).unwrap();

        assert!(SparseStorage::validate_for_insert(&vector).is_ok());
    }

    #[test]
    fn test_validate_for_insert_accepts_large_vector() {
        let indices: Vec<u32> = (0..1000).collect();
        let values: Vec<f32> = (0..1000).map(|i| i as f32 * 0.001).collect();
        let vector = SparseVector::new(indices, values, 10000).unwrap();

        assert!(SparseStorage::validate_for_insert(&vector).is_ok());
    }

    // ============= Day 2: Utility Method Tests =============

    #[test]
    fn test_total_nnz() {
        let mut storage = SparseStorage::new();

        let v1 = SparseVector::new(vec![0, 1], vec![0.1, 0.2], 10).unwrap();
        let v2 = SparseVector::new(vec![0, 1, 2], vec![0.1, 0.2, 0.3], 10).unwrap();

        storage.insert(&v1).unwrap();
        assert_eq!(storage.total_nnz(), 2);

        storage.insert(&v2).unwrap();
        assert_eq!(storage.total_nnz(), 5);
    }

    #[test]
    fn test_total_nnz_empty() {
        let storage = SparseStorage::new();
        assert_eq!(storage.total_nnz(), 0);
    }

    #[test]
    fn test_live_count_all_active() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 10).unwrap();

        storage.insert(&v).unwrap();
        storage.insert(&v).unwrap();
        storage.insert(&v).unwrap();

        assert_eq!(storage.live_count(), 3);
        assert_eq!(storage.len(), 3);
    }

    // ============= Day 3: Zero-Copy Access Tests =============

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
        assert!(storage.get_indices(SparseId::new(0)).is_none());
        assert!(storage.get_indices(SparseId::new(999)).is_none());
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

        let id1 = storage
            .insert(&SparseVector::new(vec![0], vec![1.0], 100).unwrap())
            .unwrap();
        let id2 = storage
            .insert(&SparseVector::new(vec![5, 10], vec![2.0, 3.0], 100).unwrap())
            .unwrap();

        assert_eq!(storage.get_indices(id1), Some(&[0u32][..]));
        assert_eq!(storage.get_indices(id2), Some(&[5u32, 10][..]));
    }

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
        assert!(storage.get_values(SparseId::new(0)).is_none());
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

    #[test]
    fn test_get_dim_valid() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5], vec![0.1, 0.2], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        assert_eq!(storage.get_dim(id), Some(100));
    }

    #[test]
    fn test_get_dim_out_of_bounds() {
        let storage = SparseStorage::new();
        assert_eq!(storage.get_dim(SparseId::new(0)), None);
    }

    #[test]
    fn test_get_dim_deleted() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        storage.delete(id).unwrap();
        assert_eq!(storage.get_dim(id), None);
    }

    #[test]
    fn test_zero_copy_access_matches_get() {
        let mut storage = SparseStorage::new();
        let vec = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let id = storage.insert(&vec).unwrap();

        let full_vec = storage.get(id).unwrap();
        let indices = storage.get_indices(id).unwrap();
        let values = storage.get_values(id).unwrap();
        let dim = storage.get_dim(id).unwrap();

        assert_eq!(full_vec.indices(), indices);
        assert_eq!(full_vec.values(), values);
        assert_eq!(full_vec.dim(), dim);
    }

    // ============= Day 3: Get Roundtrip Tests =============

    #[test]
    fn test_get_roundtrip_100_vectors() {
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
        storage.delete(SparseId::new(2)).unwrap();
        storage.delete(SparseId::new(5)).unwrap();
        storage.delete(SparseId::new(8)).unwrap();

        let iter_count = storage.iter().count();
        let live_count = storage.live_count();

        assert_eq!(iter_count, live_count);
        assert_eq!(iter_count, 7);
    }

    // ============= Day 4: Serialization Tests =============

    #[test]
    fn test_magic_number_constant() {
        assert_eq!(SPARSE_MAGIC, [b'E', b'S', b'P', b'V']);
        assert_eq!(&SPARSE_MAGIC, b"ESPV");
    }

    #[test]
    fn test_format_version_constant() {
        assert_eq!(SPARSE_FORMAT_VERSION, 2);
    }

    #[test]
    fn test_save_creates_file_with_magic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let storage = SparseStorage::new();
        storage.save(&path).unwrap();

        // Read first 4 bytes and verify magic
        let mut file = File::open(&path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(magic, SPARSE_MAGIC);
    }

    #[test]
    fn test_save_writes_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let storage = SparseStorage::new();
        storage.save(&path).unwrap();

        // Read bytes 4-7 and verify version
        let mut file = File::open(&path).unwrap();
        let mut buf = [0u8; 8];
        file.read_exact(&mut buf).unwrap();
        let version = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(version, SPARSE_FORMAT_VERSION);
    }

    #[test]
    fn test_load_invalid_magic_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        // Write file with wrong magic
        let mut file = File::create(&path).unwrap();
        file.write_all(b"XXXX").unwrap();
        file.write_all(&1u32.to_le_bytes()).unwrap();
        drop(file);

        let result = SparseStorage::load(&path);
        assert!(matches!(
            result,
            Err(SparseError::InvalidMagic {
                expected: _,
                found: _
            })
        ));
    }

    #[test]
    fn test_load_unsupported_version_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        // Write file with unsupported version
        let mut file = File::create(&path).unwrap();
        file.write_all(&SPARSE_MAGIC).unwrap();
        file.write_all(&999u32.to_le_bytes()).unwrap(); // Future version
        drop(file);

        let result = SparseStorage::load(&path);
        assert!(matches!(
            result,
            Err(SparseError::UnsupportedVersion {
                expected: 2,
                found: 999
            })
        ));
    }

    #[test]
    fn test_load_truncated_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        // Write file with valid header but truncated data
        let mut file = File::create(&path).unwrap();
        file.write_all(&SPARSE_MAGIC).unwrap();
        file.write_all(&SPARSE_FORMAT_VERSION.to_le_bytes())
            .unwrap();
        // Missing count and other data
        drop(file);

        let result = SparseStorage::load(&path);
        // v2 format detects truncation during CRC32 validation
        assert!(result.is_err());
    }

    #[test]
    fn test_pack_unpack_deleted_bits_empty() {
        let storage = SparseStorage::new();
        let packed = storage.pack_deleted_bits();
        assert!(packed.is_empty());

        let unpacked = SparseStorage::unpack_deleted_bits(&packed, 0);
        assert!(unpacked.is_empty());
    }

    #[test]
    fn test_pack_unpack_deleted_bits_roundtrip() {
        let mut storage = SparseStorage::new();

        // Insert some vectors
        let v1 = SparseVector::singleton(0, 1.0, 100).unwrap();
        let v2 = SparseVector::singleton(1, 2.0, 100).unwrap();
        let v3 = SparseVector::singleton(2, 3.0, 100).unwrap();

        storage.insert(&v1).unwrap();
        let id2 = storage.insert(&v2).unwrap();
        storage.insert(&v3).unwrap();

        // Delete one
        storage.delete(id2).unwrap();

        // Pack and unpack
        let packed = storage.pack_deleted_bits();
        let unpacked = SparseStorage::unpack_deleted_bits(&packed, 3);

        assert_eq!(unpacked.len(), 3);
        assert!(!unpacked[0]); // Not deleted
        assert!(unpacked[1]); // Deleted
        assert!(!unpacked[2]); // Not deleted
    }

    // ============= Day 4: Roundtrip Tests =============

    #[test]
    fn test_roundtrip_empty_storage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let original = SparseStorage::new();
        original.save(&path).unwrap();

        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.len(), 0);
        assert_eq!(loaded.next_id(), original.next_id());
    }

    #[test]
    fn test_roundtrip_single_vector() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();
        let v = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let id = original.insert(&v).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        let recovered = loaded.get(id).unwrap();
        assert_eq!(recovered.indices(), v.indices());
        assert_eq!(recovered.values(), v.values());
        assert_eq!(recovered.dim(), v.dim());
    }

    #[test]
    fn test_roundtrip_multiple_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();
        let vectors = [
            SparseVector::new(vec![0, 5], vec![0.1, 0.2], 100).unwrap(),
            SparseVector::new(vec![1, 2, 3], vec![1.0, 2.0, 3.0], 50).unwrap(),
            SparseVector::singleton(99, 9.9, 100).unwrap(),
        ];

        let ids: Vec<_> = vectors
            .iter()
            .map(|v| original.insert(v).unwrap())
            .collect();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.len(), 3);
        for (id, expected) in ids.iter().zip(vectors.iter()) {
            let recovered = loaded.get(*id).unwrap();
            assert_eq!(recovered.indices(), expected.indices());
            assert_eq!(recovered.values(), expected.values());
            assert_eq!(recovered.dim(), expected.dim());
        }
    }

    #[test]
    fn test_roundtrip_with_deletions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();
        let v1 = SparseVector::singleton(0, 1.0, 100).unwrap();
        let v2 = SparseVector::singleton(1, 2.0, 100).unwrap();
        let v3 = SparseVector::singleton(2, 3.0, 100).unwrap();

        let id1 = original.insert(&v1).unwrap();
        let id2 = original.insert(&v2).unwrap();
        let id3 = original.insert(&v3).unwrap();

        // Delete middle vector
        original.delete(id2).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        // Active count should be 2
        assert_eq!(loaded.live_count(), 2);

        // id1 and id3 should exist
        assert!(loaded.get(id1).is_some());
        assert!(loaded.get(id2).is_none()); // Deleted
        assert!(loaded.get(id3).is_some());
    }

    #[test]
    fn test_roundtrip_preserves_next_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        original.insert(&v).unwrap();
        original.insert(&v).unwrap();
        original.insert(&v).unwrap();

        let expected_next_id = original.next_id();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.next_id(), expected_next_id);
    }

    #[test]
    fn test_roundtrip_large_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();

        // Create vector with 500 non-zeros
        let indices: Vec<u32> = (0..500).map(|i| i * 20).collect();
        let values: Vec<f32> = (0..500).map(|i| i as f32 * 0.01).collect();
        let v = SparseVector::new(indices.clone(), values.clone(), 10000).unwrap();

        let id = original.insert(&v).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        let recovered = loaded.get(id).unwrap();
        assert_eq!(recovered.indices(), &indices);
        assert_eq!(recovered.values(), &values);
        assert_eq!(recovered.nnz(), 500);
    }

    #[test]
    fn test_roundtrip_edge_case_dims() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();

        // Min dim
        let v1 = SparseVector::singleton(0, 1.0, 1).unwrap();
        // Large dim
        let v2 = SparseVector::singleton(999_999, 1.0, 1_000_000).unwrap();

        let id1 = original.insert(&v1).unwrap();
        let id2 = original.insert(&v2).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.get(id1).unwrap().dim(), 1);
        assert_eq!(loaded.get(id2).unwrap().dim(), 1_000_000);
    }

    // ============= Day 5: Deletion Support Tests =============

    #[test]
    fn test_is_deleted_false_before_delete() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        let id = storage.insert(&v).unwrap();

        assert!(!storage.is_deleted(id));
    }

    #[test]
    fn test_is_deleted_true_after_delete() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        let id = storage.insert(&v).unwrap();

        storage.delete(id).unwrap();
        assert!(storage.is_deleted(id));
    }

    #[test]
    fn test_is_deleted_true_for_nonexistent() {
        let storage = SparseStorage::new();
        assert!(storage.is_deleted(SparseId::new(999)));
    }

    #[test]
    fn test_exists_active_vector() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        let id = storage.insert(&v).unwrap();

        assert!(storage.exists(id));
        assert!(!storage.is_deleted(id));
    }

    #[test]
    fn test_exists_deleted_vector() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        let id = storage.insert(&v).unwrap();

        storage.delete(id).unwrap();

        // Deleted vector still exists, but is_deleted is true
        assert!(storage.exists(id));
        assert!(storage.is_deleted(id));
    }

    #[test]
    fn test_exists_nonexistent() {
        let storage = SparseStorage::new();
        let fake_id = SparseId::new(999);

        assert!(!storage.exists(fake_id));
        assert!(storage.is_deleted(fake_id));
    }

    #[test]
    fn test_exists_vs_is_deleted_semantics() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        let id = storage.insert(&v).unwrap();
        let fake_id = SparseId::new(999);

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

    #[test]
    fn test_deleted_count_increments() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        let id1 = storage.insert(&v).unwrap();
        let id2 = storage.insert(&v).unwrap();

        assert_eq!(storage.deleted_count(), 0);

        storage.delete(id1).unwrap();
        assert_eq!(storage.deleted_count(), 1);

        storage.delete(id2).unwrap();
        assert_eq!(storage.deleted_count(), 2);
    }

    #[test]
    fn test_active_count_decrements() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        let id1 = storage.insert(&v).unwrap();
        storage.insert(&v).unwrap();

        assert_eq!(storage.active_count(), 2);

        storage.delete(id1).unwrap();
        assert_eq!(storage.active_count(), 1);
    }

    #[test]
    fn test_total_count_unchanged_after_delete() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        let id = storage.insert(&v).unwrap();
        assert_eq!(storage.total_count(), 1);

        storage.delete(id).unwrap();
        assert_eq!(storage.total_count(), 1); // Still 1 (soft delete)
    }

    #[test]
    fn test_count_invariant() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        for _ in 0..10 {
            storage.insert(&v).unwrap();
        }

        storage.delete(SparseId::new(2)).unwrap();
        storage.delete(SparseId::new(5)).unwrap();
        storage.delete(SparseId::new(8)).unwrap();

        // Invariant: active + deleted = total
        assert_eq!(
            storage.active_count() + storage.deleted_count(),
            storage.total_count()
        );
        assert_eq!(storage.active_count(), 7);
        assert_eq!(storage.deleted_count(), 3);
        assert_eq!(storage.total_count(), 10);
    }

    #[test]
    fn test_deletion_ratio_empty() {
        let storage = SparseStorage::new();
        assert_eq!(storage.deletion_ratio(), 0.0);
    }

    #[test]
    fn test_deletion_ratio_no_deletions() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        for _ in 0..4 {
            storage.insert(&v).unwrap();
        }

        assert_eq!(storage.deletion_ratio(), 0.0);
    }

    #[test]
    fn test_deletion_ratio_some_deleted() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        for _ in 0..4 {
            storage.insert(&v).unwrap();
        }

        // Delete 1 of 4 = 25%
        storage.delete(SparseId::new(0)).unwrap();
        assert!((storage.deletion_ratio() - 0.25).abs() < 0.01);

        // Delete 2 of 4 = 50%
        storage.delete(SparseId::new(1)).unwrap();
        assert!((storage.deletion_ratio() - 0.50).abs() < 0.01);
    }

    #[test]
    fn test_deletion_ratio_all_deleted() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        let id1 = storage.insert(&v).unwrap();
        let id2 = storage.insert(&v).unwrap();

        storage.delete(id1).unwrap();
        storage.delete(id2).unwrap();

        assert!((storage.deletion_ratio() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_iter_all_deleted() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        let id1 = storage.insert(&v).unwrap();
        let id2 = storage.insert(&v).unwrap();

        storage.delete(id1).unwrap();
        storage.delete(id2).unwrap();

        let count = storage.iter().count();
        assert_eq!(count, 0, "All deleted, iter should be empty");
    }

    #[test]
    fn test_ids_all_deleted() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        let id1 = storage.insert(&v).unwrap();
        let id2 = storage.insert(&v).unwrap();

        storage.delete(id1).unwrap();
        storage.delete(id2).unwrap();

        let count = storage.ids().count();
        assert_eq!(count, 0, "All deleted, ids should be empty");
    }

    #[test]
    fn test_delete_and_insert_interleaved() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        // Insert 3
        let id1 = storage.insert(&v).unwrap();
        let id2 = storage.insert(&v).unwrap();
        let id3 = storage.insert(&v).unwrap();

        // Delete middle
        storage.delete(id2).unwrap();

        // Insert more
        let id4 = storage.insert(&v).unwrap();

        // Verify state
        assert_eq!(storage.total_count(), 4);
        assert_eq!(storage.active_count(), 3);
        assert_eq!(storage.deleted_count(), 1);

        // Verify which IDs are active
        assert!(storage.contains(id1));
        assert!(!storage.contains(id2));
        assert!(storage.contains(id3));
        assert!(storage.contains(id4));
    }

    #[test]
    fn test_iter_count_equals_active_count() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();

        for _ in 0..20 {
            storage.insert(&v).unwrap();
        }

        // Delete some
        storage.delete(SparseId::new(5)).unwrap();
        storage.delete(SparseId::new(10)).unwrap();
        storage.delete(SparseId::new(15)).unwrap();

        assert_eq!(storage.iter().count(), storage.active_count());
        assert_eq!(storage.ids().count(), storage.active_count());
    }

    // ============= Phase 3 Remediation Tests =============

    #[test]
    fn test_crc32_checksum_present_in_v2() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        storage.insert(&v).unwrap();

        let bytes = storage.to_bytes();
        // Last 4 bytes should be the CRC32 checksum
        assert!(bytes.len() >= 12); // at least header + checksum
        let payload = &bytes[8..bytes.len() - 4];
        let expected_crc = crc32fast::hash(payload);
        let stored_crc = u32::from_le_bytes([
            bytes[bytes.len() - 4],
            bytes[bytes.len() - 3],
            bytes[bytes.len() - 2],
            bytes[bytes.len() - 1],
        ]);
        assert_eq!(expected_crc, stored_crc);
    }

    #[test]
    fn test_crc32_detects_corruption() {
        let mut storage = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        storage.insert(&v).unwrap();

        let mut bytes = storage.to_bytes();
        // Corrupt a byte in the payload (not in header or checksum)
        if bytes.len() > 12 {
            bytes[10] ^= 0xFF;
        }

        let result = SparseStorage::from_bytes(&bytes);
        assert!(matches!(result, Err(SparseError::CorruptedData(_))));
    }

    #[test]
    fn test_crc32_roundtrip_via_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_crc.espv");

        let mut original = SparseStorage::new();
        let v = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        original.insert(&v).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.len(), original.len());
    }

    #[test]
    fn test_id_overflow_protection() {
        let mut storage = SparseStorage::new();
        // Set next_id to u64::MAX to trigger overflow on next insert
        storage.next_id = u64::MAX;

        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        let result = storage.insert(&v);
        assert!(matches!(result, Err(SparseError::IdOverflow)));
    }
}

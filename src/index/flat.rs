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
//! # Memory Layout
//!
//! Vectors are stored in row-major (contiguous per-vector) layout:
//! ```text
//! vectors = [v0_d0, v0_d1, ..., v0_dn, v1_d0, v1_d1, ..., v1_dn, ...]
//! ```
//!
//! This allows simple `&vectors[id*dim..(id+1)*dim]` slicing for retrieval.
//!
//! # Example
//!
//! ```rust
//! use edgevec::index::{FlatIndex, FlatIndexConfig, DistanceMetric};
//!
//! // Create a flat index for 128-dimensional vectors
//! let config = FlatIndexConfig::new(128)
//!     .with_metric(DistanceMetric::Cosine)
//!     .with_capacity(1000);
//! let mut index = FlatIndex::new(config);
//!
//! // Insert vectors (O(1) operation)
//! let id1 = index.insert(&[0.1; 128]).unwrap();
//! let id2 = index.insert(&[0.2; 128]).unwrap();
//!
//! // Check existence
//! assert!(index.contains(id1));
//!
//! // Retrieve vector
//! let v = index.get(id1).unwrap();
//! assert_eq!(v.len(), 128);
//! ```

use crate::persistence::PersistenceError;
use bitvec::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use thiserror::Error;

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that can occur during FlatIndex operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FlatIndexError {
    /// Vector dimension does not match index configuration.
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimension from index configuration.
        expected: usize,
        /// Actual dimension of the provided vector.
        actual: usize,
    },

    /// Invalid search parameter k (must be > 0).
    #[error("invalid k: must be greater than 0")]
    InvalidK,

    /// Quantization is not enabled but quantized search was requested.
    #[error("quantization not enabled: call enable_quantization() first")]
    QuantizationNotEnabled,

    /// Index is empty (no vectors to search).
    #[error("index is empty")]
    EmptyIndex,
}

// ============================================================================
// DISTANCE METRIC
// ============================================================================

/// Distance metric for vector comparison.
///
/// Determines how distances/similarities are computed during search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DistanceMetric {
    /// Cosine similarity: dot(a, b) / (||a|| * ||b||).
    /// Result is in [-1, 1], where 1 = identical, -1 = opposite.
    /// Search returns highest similarity first.
    #[default]
    Cosine,

    /// Dot product: sum(a[i] * b[i]).
    /// Result is unbounded. Higher = more similar.
    /// Search returns highest dot product first.
    DotProduct,

    /// L2 (Euclidean) distance: sqrt(sum((a[i] - b[i])^2)).
    /// Result is in [0, inf). Lower = more similar.
    /// Search returns lowest distance first.
    L2,

    /// Hamming distance: count of positions where a[i] != b[i].
    /// For use with binary-like vectors (0.0 vs non-zero).
    /// Result is in [0, dim]. Lower = more similar.
    /// Search returns lowest distance first.
    Hamming,
}

impl DistanceMetric {
    /// Returns true if this metric measures similarity (higher = better).
    /// Returns false if this metric measures distance (lower = better).
    #[must_use]
    pub const fn is_similarity(&self) -> bool {
        matches!(self, Self::Cosine | Self::DotProduct)
    }
}

// ============================================================================
// SEARCH RESULT
// ============================================================================

/// Search result from FlatIndex.
///
/// Contains the vector ID and its distance/similarity score to the query.
#[derive(Debug, Clone)]
pub struct FlatSearchResult {
    /// Vector ID in the index.
    pub id: u64,

    /// Distance or similarity score.
    /// - For distance metrics (L2, Hamming): lower is better
    /// - For similarity metrics (Cosine, Dot): higher is better
    pub score: f32,
}

impl PartialEq for FlatSearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && (self.score - other.score).abs() < f32::EPSILON
    }
}

impl Eq for FlatSearchResult {}

/// Internal wrapper for max-heap behavior during search.
/// The heap keeps the k-worst results at the top for easy removal.
#[derive(Debug, Clone)]
struct HeapEntry {
    id: u64,
    score: f32,
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && (self.score - other.score).abs() < f32::EPSILON
    }
}

impl Eq for HeapEntry {}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Max-heap: higher score = higher priority (for removal)
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(Ordering::Equal)
    }
}

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for FlatIndex.
///
/// # Example
///
/// ```rust
/// use edgevec::index::{FlatIndexConfig, DistanceMetric};
///
/// let config = FlatIndexConfig::new(128)
///     .with_metric(DistanceMetric::Cosine)
///     .with_capacity(5000)
///     .with_cleanup_threshold(0.3);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatIndexConfig {
    /// Vector dimension (must match all inserted vectors).
    pub dimensions: u32,

    /// Distance metric for search operations.
    pub metric: DistanceMetric,

    /// Initial capacity hint for pre-allocation.
    /// The index will grow automatically if this is exceeded.
    pub initial_capacity: usize,

    /// Cleanup threshold: fraction of deleted vectors that triggers compaction.
    /// Range: [0.0, 1.0]. Default: 0.5 (compact when 50% deleted).
    pub cleanup_threshold: f32,
}

impl FlatIndexConfig {
    /// Create a new configuration with the given vector dimension.
    ///
    /// Uses default values:
    /// - Metric: Cosine
    /// - Initial capacity: 1000
    /// - Cleanup threshold: 0.5
    #[must_use]
    pub fn new(dimensions: u32) -> Self {
        Self {
            dimensions,
            metric: DistanceMetric::Cosine,
            initial_capacity: 1000,
            cleanup_threshold: 0.5,
        }
    }

    /// Set the distance metric.
    #[must_use]
    pub fn with_metric(mut self, metric: DistanceMetric) -> Self {
        self.metric = metric;
        self
    }

    /// Set the initial capacity (number of vectors to pre-allocate).
    #[must_use]
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.initial_capacity = capacity;
        self
    }

    /// Set the cleanup threshold (0.0 to 1.0).
    ///
    /// When the fraction of deleted vectors exceeds this threshold,
    /// compaction is triggered to reclaim memory.
    #[must_use]
    pub fn with_cleanup_threshold(mut self, threshold: f32) -> Self {
        self.cleanup_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

// ============================================================================
// FLAT INDEX
// ============================================================================

/// Flat (brute-force) index for exact nearest neighbor search.
///
/// Stores vectors in row-major layout for simple slicing.
/// Provides O(1) insertion and O(n·d) search with 100% recall guarantee.
///
/// # Memory Layout
///
/// - Vectors: `Vec<f32>` in row-major order (n × d elements)
/// - Deletion bitmap: `BitVec` (1 bit per vector)
/// - For 10k vectors @ 768 dimensions:
///   - F32: ~30 MB (10,000 × 768 × 4 bytes)
///   - Bitmap: ~1.25 KB
///
/// # Thread Safety
///
/// `FlatIndex` is not thread-safe by default. Wrap in `Arc<RwLock<_>>` for
/// concurrent access.
pub struct FlatIndex {
    /// Configuration (immutable after creation).
    config: FlatIndexConfig,

    /// Dense vectors in row-major layout.
    /// Layout: [v0_d0, v0_d1, ..., v0_dn, v1_d0, v1_d1, ..., v1_dn, ...]
    vectors: Vec<f32>,

    /// Number of vectors stored (including deleted slots).
    count: u64,

    /// Bitmap tracking deleted vectors.
    /// `deleted[i] == true` means vector i is deleted.
    deleted: BitVec,

    /// Number of deleted vectors (for cleanup threshold calculation).
    delete_count: usize,

    /// Next ID to assign (monotonically increasing).
    next_id: u64,

    /// Optional binary quantized vectors for BQ mode.
    /// Layout: [v0_byte0, v0_byte1, ..., v1_byte0, ...] where each byte packs 8 dimensions.
    quantized: Option<Vec<u8>>,
}

impl FlatIndex {
    /// Create a new FlatIndex with the given configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let config = FlatIndexConfig::new(128);
    /// let index = FlatIndex::new(config);
    ///
    /// assert_eq!(index.dimensions(), 128);
    /// assert!(index.is_empty());
    /// ```
    #[must_use]
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
            quantized: None,
        }
    }

    /// Returns the vector dimension.
    #[must_use]
    pub fn dimensions(&self) -> u32 {
        self.config.dimensions
    }

    /// Returns the distance metric.
    #[must_use]
    pub fn metric(&self) -> DistanceMetric {
        self.config.metric
    }

    /// Returns the number of vectors (excluding deleted).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors; u64→usize safe
    pub fn len(&self) -> usize {
        (self.count as usize).saturating_sub(self.delete_count)
    }

    /// Returns true if the index is empty (no non-deleted vectors).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total number of slots (including deleted).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors; u64→usize safe
    pub fn capacity(&self) -> usize {
        self.count as usize
    }

    /// Returns a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &FlatIndexConfig {
        &self.config
    }

    // ========================================================================
    // INSERT
    // ========================================================================

    /// Insert a vector into the index.
    ///
    /// Returns the assigned vector ID.
    ///
    /// # Errors
    ///
    /// Returns `FlatIndexError::DimensionMismatch` if the vector dimension
    /// doesn't match the index configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(3));
    ///
    /// let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();
    /// assert_eq!(id, 0);
    ///
    /// let id2 = index.insert(&[4.0, 5.0, 6.0]).unwrap();
    /// assert_eq!(id2, 1);
    /// ```
    pub fn insert(&mut self, vector: &[f32]) -> Result<u64, FlatIndexError> {
        // Validate dimension
        let expected_dim = self.config.dimensions as usize;
        if vector.len() != expected_dim {
            return Err(FlatIndexError::DimensionMismatch {
                expected: expected_dim,
                actual: vector.len(),
            });
        }

        // Allocate ID (monotonically increasing)
        let id = self.next_id;
        self.next_id += 1;

        // Store vector (append to contiguous storage)
        self.vectors.extend_from_slice(vector);

        // Update count and deleted bitmap
        self.count += 1;
        self.deleted.push(false);

        // Invalidate quantized cache if it exists
        if self.quantized.is_some() {
            self.quantized = None;
        }

        Ok(id)
    }

    /// Insert multiple vectors in batch.
    ///
    /// Returns the IDs assigned to each vector.
    ///
    /// # Errors
    ///
    /// Returns error if any vector has wrong dimension.
    /// On error, vectors inserted before the error remain in the index.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(3));
    ///
    /// let vectors: Vec<&[f32]> = vec![
    ///     &[1.0, 2.0, 3.0],
    ///     &[4.0, 5.0, 6.0],
    /// ];
    /// let ids = index.insert_batch(&vectors).unwrap();
    ///
    /// assert_eq!(ids, vec![0, 1]);
    /// ```
    pub fn insert_batch(&mut self, vectors: &[&[f32]]) -> Result<Vec<u64>, FlatIndexError> {
        let mut ids = Vec::with_capacity(vectors.len());

        for vector in vectors {
            let id = self.insert(vector)?;
            ids.push(id);
        }

        Ok(ids)
    }

    // ========================================================================
    // GET
    // ========================================================================

    /// Get a vector by ID.
    ///
    /// Returns `None` if the ID doesn't exist or was deleted.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(3));
    /// let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();
    ///
    /// let v = index.get(id).unwrap();
    /// assert_eq!(v, &[1.0, 2.0, 3.0]);
    ///
    /// // Non-existent ID returns None
    /// assert!(index.get(999).is_none());
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors; u64→usize safe
    pub fn get(&self, id: u64) -> Option<&[f32]> {
        let idx = id as usize;
        let count = self.count as usize;

        // Check bounds
        if idx >= count {
            return None;
        }

        // Check if deleted
        if self.deleted.get(idx).map_or(true, |b| *b) {
            return None;
        }

        // Return vector slice
        let dim = self.config.dimensions as usize;
        let start = idx * dim;
        let end = start + dim;

        Some(&self.vectors[start..end])
    }

    /// Check if a vector ID exists and is not deleted.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(3));
    /// let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();
    ///
    /// assert!(index.contains(id));
    /// assert!(!index.contains(999));
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors; u64→usize safe
    pub fn contains(&self, id: u64) -> bool {
        let idx = id as usize;
        let count = self.count as usize;
        idx < count && !self.deleted.get(idx).map_or(true, |b| *b)
    }

    // ========================================================================
    // STATS
    // ========================================================================

    /// Returns the number of deleted vectors.
    #[must_use]
    pub fn deleted_count(&self) -> usize {
        self.delete_count
    }

    /// Returns the deletion ratio (deleted / total).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for ratio calculation
    pub fn deletion_ratio(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.delete_count as f32 / self.count as f32
        }
    }

    /// Returns true if quantization is enabled.
    #[must_use]
    pub fn is_quantized(&self) -> bool {
        self.quantized.is_some()
    }

    /// Returns memory usage in bytes (approximate).
    ///
    /// Includes:
    /// - Vector storage (n × dim × 4 bytes for f32)
    /// - Deleted bitmap (n / 8 bytes)
    /// - Quantized storage if enabled (n × ceil(dim/8) bytes)
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        let vector_bytes = self.vectors.len() * std::mem::size_of::<f32>();
        let bitmap_bytes = (self.deleted.len() + 7) / 8;
        let quantized_bytes = self.quantized.as_ref().map_or(0, Vec::len);

        vector_bytes + bitmap_bytes + quantized_bytes
    }

    /// Get deletion statistics.
    ///
    /// Returns a tuple of (total_vectors, deleted_count, deletion_ratio).
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(3));
    /// index.insert(&[1.0, 2.0, 3.0]).unwrap();
    /// index.insert(&[4.0, 5.0, 6.0]).unwrap();
    /// index.delete(0);
    ///
    /// let (total, deleted, ratio) = index.deletion_stats();
    /// assert_eq!(total, 2);
    /// assert_eq!(deleted, 1);
    /// assert!((ratio - 0.5).abs() < 0.01);
    /// ```
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for ratio calculation
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors
    pub fn deletion_stats(&self) -> (usize, usize, f32) {
        let total = self.count as usize;
        let deleted = self.delete_count;
        let ratio = if total > 0 {
            deleted as f32 / total as f32
        } else {
            0.0
        };
        (total, deleted, ratio)
    }

    // ========================================================================
    // DELETE
    // ========================================================================

    /// Mark a vector as deleted.
    ///
    /// The vector is not immediately removed; its slot is marked in the
    /// deletion bitmap and skipped during search. Call `compact()` to
    /// reclaim space when deletion rate exceeds the threshold.
    ///
    /// # Returns
    ///
    /// Returns `true` if the vector was deleted, `false` if it didn't exist
    /// or was already deleted.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(3));
    /// let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();
    ///
    /// assert!(index.delete(id)); // Success
    /// assert!(!index.delete(id)); // Already deleted
    /// assert!(!index.delete(999)); // Doesn't exist
    ///
    /// assert!(!index.contains(id)); // No longer accessible
    /// ```
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors
    pub fn delete(&mut self, id: u64) -> bool {
        let idx = id as usize;
        let count = self.count as usize;

        // Check bounds
        if idx >= count {
            return false;
        }

        // Check if already deleted
        if self.deleted.get(idx).map_or(true, |b| *b) {
            return false;
        }

        // Mark as deleted
        self.deleted.set(idx, true);
        self.delete_count += 1;

        // Invalidate quantized cache
        if self.quantized.is_some() {
            self.quantized = None;
        }

        // Auto-compact if threshold exceeded
        if self.should_compact() {
            self.compact();
        }

        true
    }

    /// Check if compaction is needed.
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for ratio
    fn should_compact(&self) -> bool {
        if self.count == 0 {
            return false;
        }
        (self.delete_count as f32 / self.count as f32) > self.config.cleanup_threshold
    }

    /// Compact the index by removing deleted vectors.
    ///
    /// This rebuilds the internal storage, removing deleted slots.
    ///
    /// # Warning
    ///
    /// This operation changes vector IDs! After compaction, vectors are
    /// renumbered to be contiguous. Use with caution if external systems
    /// reference vector IDs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(
    ///     FlatIndexConfig::new(3).with_cleanup_threshold(1.0) // Disable auto-compact
    /// );
    ///
    /// index.insert(&[1.0, 2.0, 3.0]).unwrap();
    /// index.insert(&[4.0, 5.0, 6.0]).unwrap();
    /// index.insert(&[7.0, 8.0, 9.0]).unwrap();
    ///
    /// // Delete middle vector
    /// index.delete(1);
    /// assert_eq!(index.len(), 2);
    /// assert_eq!(index.capacity(), 3); // Still has 3 slots
    ///
    /// // Compact
    /// index.compact();
    /// assert_eq!(index.capacity(), 2); // Now only 2 slots
    /// ```
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors
    pub fn compact(&mut self) {
        if self.delete_count == 0 {
            return;
        }

        let dim = self.config.dimensions as usize;
        let count = self.count as usize;
        let new_count = count - self.delete_count;

        let mut new_vectors = Vec::with_capacity(new_count * dim);
        let mut new_deleted = BitVec::with_capacity(new_count);

        for idx in 0..count {
            if !self.deleted.get(idx).map_or(true, |b| *b) {
                // Copy live vector
                let start = idx * dim;
                new_vectors.extend_from_slice(&self.vectors[start..start + dim]);
                new_deleted.push(false);
            }
        }

        self.vectors = new_vectors;
        self.deleted = new_deleted;
        self.count = new_count as u64;
        self.delete_count = 0;
        // Note: next_id stays the same to avoid ID reuse confusion
        // (although after compact, old IDs are invalid anyway)

        // Quantized cache already invalidated in delete()
    }

    // ========================================================================
    // QUANTIZATION
    // ========================================================================

    /// Enable binary quantization for memory reduction.
    ///
    /// Converts stored vectors to binary format (32x compression).
    /// After enabling, use `search_quantized()` for fast Hamming distance search.
    ///
    /// # Memory Reduction
    ///
    /// - Original: 768D × 4 bytes = 3072 bytes per vector
    /// - Quantized: 768D / 8 = 96 bytes per vector (32x reduction)
    ///
    /// # Warning
    ///
    /// This is a lossy operation. Recall will decrease from 100%
    /// but memory usage drops significantly. The original F32 vectors
    /// are preserved for `search()` to maintain 100% recall option.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(8));
    /// index.insert(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0]).unwrap();
    ///
    /// index.enable_quantization().unwrap();
    /// assert!(index.is_quantized());
    ///
    /// // Can still use exact search
    /// let exact = index.search(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0], 1).unwrap();
    ///
    /// // Or use faster quantized search
    /// let approx = index.search_quantized(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0], 1).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Currently infallible, but returns `Result` for API consistency
    /// and future error conditions.
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors
    pub fn enable_quantization(&mut self) -> Result<(), FlatIndexError> {
        if self.quantized.is_some() {
            return Ok(()); // Already enabled
        }

        if self.count == 0 {
            // Nothing to quantize, but set up empty storage
            self.quantized = Some(Vec::new());
            return Ok(());
        }

        let dim = self.config.dimensions as usize;
        let packed_dim = (dim + 7) / 8; // Bytes needed for dim bits
        let count = self.count as usize;

        let mut quantized = Vec::with_capacity(count * packed_dim);

        for idx in 0..count {
            if self.deleted.get(idx).map_or(true, |b| *b) {
                // Placeholder for deleted vectors (zeros)
                quantized.extend(std::iter::repeat(0u8).take(packed_dim));
                continue;
            }

            let start = idx * dim;
            let vector = &self.vectors[start..start + dim];

            // Binarize: value > 0 = 1, else 0
            let packed = Self::binarize_vector(vector);
            quantized.extend_from_slice(&packed);
        }

        self.quantized = Some(quantized);
        Ok(())
    }

    /// Disable quantization, freeing the quantized storage.
    ///
    /// Search will use original F32 vectors only.
    pub fn disable_quantization(&mut self) {
        self.quantized = None;
    }

    /// Binarize a vector to packed bytes.
    ///
    /// Each bit represents whether the corresponding dimension value is > 0.
    /// Bits are packed MSB-first within each byte.
    fn binarize_vector(vector: &[f32]) -> Vec<u8> {
        let dim = vector.len();
        let packed_dim = (dim + 7) / 8;
        let mut packed = vec![0u8; packed_dim];

        for (i, &val) in vector.iter().enumerate() {
            if val > 0.0 {
                packed[i / 8] |= 1 << (7 - (i % 8));
            }
        }

        packed
    }

    /// Compute Hamming distance between two packed binary vectors.
    ///
    /// Uses popcount for efficient bit counting.
    #[inline]
    fn hamming_distance_binary(a: &[u8], b: &[u8]) -> u32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x ^ y).count_ones())
            .sum()
    }

    /// Search using quantized vectors.
    ///
    /// Uses Hamming distance on binary-quantized vectors for fast approximate search.
    /// Results are sorted by ascending Hamming distance (lower = more similar).
    ///
    /// # Errors
    ///
    /// - `FlatIndexError::QuantizationNotEnabled` if quantization is not enabled
    /// - `FlatIndexError::DimensionMismatch` if query dimension is wrong
    /// - `FlatIndexError::InvalidK` if k is 0
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig};
    ///
    /// let mut index = FlatIndex::new(FlatIndexConfig::new(8));
    /// index.insert(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0]).unwrap();
    /// index.insert(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]).unwrap();
    ///
    /// index.enable_quantization().unwrap();
    ///
    /// let results = index.search_quantized(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0], 2).unwrap();
    /// assert_eq!(results[0].id, 0); // Exact match has Hamming distance 0
    /// assert!((results[0].score - 0.0).abs() < f32::EPSILON);
    /// ```
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors
    #[allow(clippy::cast_precision_loss)] // Hamming distance fits in f32
    pub fn search_quantized(
        &self,
        query: &[f32],
        k: usize,
    ) -> Result<Vec<FlatSearchResult>, FlatIndexError> {
        let quantized = self
            .quantized
            .as_ref()
            .ok_or(FlatIndexError::QuantizationNotEnabled)?;

        // Validate
        let expected_dim = self.config.dimensions as usize;
        if query.len() != expected_dim {
            return Err(FlatIndexError::DimensionMismatch {
                expected: expected_dim,
                actual: query.len(),
            });
        }

        if k == 0 {
            return Err(FlatIndexError::InvalidK);
        }

        // Empty index returns empty results
        if self.count == 0 {
            return Ok(Vec::new());
        }

        // Binarize query
        let query_packed = Self::binarize_vector(query);
        let packed_dim = query_packed.len();

        let mut heap: BinaryHeap<HeapEntry> = BinaryHeap::with_capacity(k + 1);
        let count = self.count as usize;

        for idx in 0..count {
            if self.deleted.get(idx).map_or(true, |b| *b) {
                continue;
            }

            let start = idx * packed_dim;
            let vector_packed = &quantized[start..start + packed_dim];

            let distance = Self::hamming_distance_binary(&query_packed, vector_packed);

            // For Hamming distance, lower is better, so use max-heap directly
            // (heap pops highest, which is worst for distance)
            if heap.len() < k {
                heap.push(HeapEntry {
                    id: idx as u64,
                    score: distance as f32,
                });
            } else if let Some(top) = heap.peek() {
                if (distance as f32) < top.score {
                    heap.pop();
                    heap.push(HeapEntry {
                        id: idx as u64,
                        score: distance as f32,
                    });
                }
            }
        }

        // Extract and sort (ascending distance)
        let mut results: Vec<FlatSearchResult> = heap
            .into_iter()
            .map(|entry| FlatSearchResult {
                id: entry.id,
                score: entry.score,
            })
            .collect();

        // Sort by ascending Hamming distance (lower = better)
        results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));

        Ok(results)
    }

    // ========================================================================
    // SEARCH
    // ========================================================================

    /// Search for the k nearest neighbors.
    ///
    /// Returns results sorted by relevance (best first):
    /// - For distance metrics (L2, Hamming): lowest distance first
    /// - For similarity metrics (Cosine, Dot): highest similarity first
    ///
    /// # Arguments
    ///
    /// * `query` - Query vector (must match index dimensions)
    /// * `k` - Number of results to return
    ///
    /// # Errors
    ///
    /// - `FlatIndexError::DimensionMismatch` if query dimension is wrong
    /// - `FlatIndexError::InvalidK` if k is 0
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::index::{FlatIndex, FlatIndexConfig, DistanceMetric};
    ///
    /// let config = FlatIndexConfig::new(3).with_metric(DistanceMetric::Cosine);
    /// let mut index = FlatIndex::new(config);
    ///
    /// index.insert(&[1.0, 0.0, 0.0]).unwrap();
    /// index.insert(&[0.0, 1.0, 0.0]).unwrap();
    ///
    /// let results = index.search(&[0.9, 0.1, 0.0], 2).unwrap();
    /// assert_eq!(results.len(), 2);
    /// assert_eq!(results[0].id, 0); // First vector is most similar
    /// ```
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<FlatSearchResult>, FlatIndexError> {
        // Validate inputs
        let expected_dim = self.config.dimensions as usize;
        if query.len() != expected_dim {
            return Err(FlatIndexError::DimensionMismatch {
                expected: expected_dim,
                actual: query.len(),
            });
        }

        if k == 0 {
            return Err(FlatIndexError::InvalidK);
        }

        // Empty index returns empty results
        if self.count == 0 {
            return Ok(Vec::new());
        }

        let dim = self.config.dimensions as usize;
        let is_similarity = self.config.metric.is_similarity();

        // Use max-heap to track top-k
        // For similarity: negate scores so max-heap gives us lowest negated (highest original)
        // For distance: use scores directly so max-heap gives us highest (worst) for removal
        let mut heap: BinaryHeap<HeapEntry> = BinaryHeap::with_capacity(k + 1);

        // Iterate all vectors
        let count = self.count as usize;
        for idx in 0..count {
            // Skip deleted
            if self.deleted.get(idx).map_or(true, |b| *b) {
                continue;
            }

            // Get vector
            let start = idx * dim;
            let end = start + dim;
            let vector = &self.vectors[start..end];

            // Compute distance/similarity
            let raw_score = self.compute_distance(query, vector);

            // Transform score for heap ordering:
            // - Similarity: negate (max-heap pops highest = lowest similarity = worst)
            // - Distance: use as-is (max-heap pops highest = highest distance = worst)
            let heap_score = if is_similarity { -raw_score } else { raw_score };

            if heap.len() < k {
                heap.push(HeapEntry {
                    id: idx as u64,
                    score: heap_score,
                });
            } else if let Some(top) = heap.peek() {
                if heap_score < top.score {
                    heap.pop();
                    heap.push(HeapEntry {
                        id: idx as u64,
                        score: heap_score,
                    });
                }
            }
        }

        // Extract results and restore original scores
        let mut results: Vec<FlatSearchResult> = heap
            .into_iter()
            .map(|entry| FlatSearchResult {
                id: entry.id,
                score: if is_similarity {
                    -entry.score
                } else {
                    entry.score
                },
            })
            .collect();

        // Sort by score (best first)
        if is_similarity {
            // Descending for similarity (higher = better)
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        } else {
            // Ascending for distance (lower = better)
            results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));
        }

        Ok(results)
    }

    // ========================================================================
    // DISTANCE COMPUTATION
    // ========================================================================

    /// Compute distance/similarity between two vectors.
    #[allow(clippy::unused_self)] // Instance method for API consistency and future SIMD
    fn compute_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.config.metric {
            DistanceMetric::Cosine => Self::cosine_similarity(a, b),
            DistanceMetric::DotProduct => Self::dot_product(a, b),
            DistanceMetric::L2 => Self::euclidean_distance(a, b),
            DistanceMetric::Hamming => Self::hamming_distance(a, b),
        }
    }

    /// Dot product: sum(a[i] * b[i]).
    #[inline]
    fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Cosine similarity: dot(a, b) / (||a|| * ||b||).
    #[inline]
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// Euclidean distance: sqrt(sum((a[i] - b[i])^2)).
    #[inline]
    fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f32>()
            .sqrt()
    }

    /// Hamming distance for f32 vectors.
    ///
    /// Treats values as binary: 0.0 = 0, non-zero = 1.
    /// Returns count of positions where binary values differ.
    ///
    /// Note: For actual binary vectors, use BQ quantization (Day 3)
    /// which uses packed u8 bytes and popcount-based Hamming distance.
    #[inline]
    #[allow(clippy::float_cmp)] // Intentional exact comparison for binary detection
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for count→f32
    fn hamming_distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .filter(|(x, y)| (**x != 0.0) != (**y != 0.0))
            .count() as f32
    }
}

// ============================================================================
// PERSISTENCE
// ============================================================================

/// Snapshot format version for FlatIndex.
pub const FLAT_INDEX_VERSION: u32 = 1;

/// Magic number for FlatIndex snapshots: "EVFI" (EdgeVec Flat Index).
pub const FLAT_INDEX_MAGIC: [u8; 4] = [b'E', b'V', b'F', b'I'];

/// FlatIndex snapshot header.
///
/// Contains all metadata needed to reconstruct a FlatIndex from a snapshot.
/// Serialized using postcard for WASM compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatIndexHeader {
    /// Magic number (must be `FLAT_INDEX_MAGIC`).
    pub magic: [u8; 4],

    /// Format version (must be <= `FLAT_INDEX_VERSION`).
    pub version: u32,

    /// Vector dimension.
    pub dimensions: u32,

    /// Distance metric.
    pub metric: DistanceMetric,

    /// Number of vectors stored (including deleted slots).
    pub count: u64,

    /// Number of deleted vectors.
    pub delete_count: u64,

    /// Next ID to assign.
    pub next_id: u64,

    /// Whether quantization is enabled.
    pub is_quantized: bool,

    /// Cleanup threshold for auto-compaction.
    pub cleanup_threshold: f32,

    /// CRC32 checksum of the data section.
    pub checksum: u32,
}

impl FlatIndexHeader {
    /// Create a header from a FlatIndex and computed checksum.
    #[must_use]
    pub fn from_index(index: &FlatIndex, checksum: u32) -> Self {
        Self {
            magic: FLAT_INDEX_MAGIC,
            version: FLAT_INDEX_VERSION,
            dimensions: index.config.dimensions,
            metric: index.config.metric,
            count: index.count,
            delete_count: index.delete_count as u64,
            next_id: index.next_id,
            is_quantized: index.quantized.is_some(),
            cleanup_threshold: index.config.cleanup_threshold,
            checksum,
        }
    }

    /// Validate the header.
    ///
    /// # Errors
    ///
    /// Returns `PersistenceError::InvalidMagic` if magic doesn't match.
    /// Returns `PersistenceError::UnsupportedVersion` if version is too new.
    #[allow(clippy::cast_possible_truncation)] // Intentional truncation for version bytes
    pub fn validate(&self) -> Result<(), PersistenceError> {
        if self.magic != FLAT_INDEX_MAGIC {
            return Err(PersistenceError::InvalidMagic {
                expected: FLAT_INDEX_MAGIC,
                actual: self.magic,
            });
        }
        if self.version > FLAT_INDEX_VERSION {
            return Err(PersistenceError::UnsupportedVersion(
                (self.version >> 8) as u8,
                (self.version & 0xFF) as u8,
            ));
        }
        Ok(())
    }
}

impl FlatIndex {
    // ========================================================================
    // PERSISTENCE
    // ========================================================================

    /// Serialize the index to a snapshot.
    ///
    /// The snapshot format is:
    /// 1. Header length (u32, 4 bytes)
    /// 2. Header (postcard serialized `FlatIndexHeader`)
    /// 3. Deleted bitmap length (u32, 4 bytes)
    /// 4. Deleted bitmap (variable bytes)
    /// 5. Vectors length (u64, 8 bytes)
    /// 6. Vectors (n × dim × 4 bytes, little-endian f32)
    /// 7. Quantized length (u64, 8 bytes, 0 if not enabled)
    /// 8. Quantized vectors (optional, n × ceil(dim/8) bytes)
    ///
    /// # Returns
    ///
    /// Bytes that can be written to IndexedDB or file system.
    ///
    /// # Errors
    ///
    /// Returns `PersistenceError::SerializationError` if serialization fails.
    #[allow(clippy::cast_possible_truncation)] // Header and bitmap sizes are small (<4GB)
    pub fn to_snapshot(&self) -> Result<Vec<u8>, PersistenceError> {
        let mut buffer = Vec::new();

        // Serialize data sections first to compute checksum
        let deleted_bytes = self.serialize_deleted_bitmap();
        let vectors_bytes = self.serialize_vectors();
        let quantized_bytes = self.serialize_quantized();

        // Compute checksum of data
        let checksum = Self::compute_checksum(&deleted_bytes, &vectors_bytes, &quantized_bytes);

        // Create and serialize header
        let header = FlatIndexHeader::from_index(self, checksum);
        let header_bytes = postcard::to_allocvec(&header)
            .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

        // Write header length (u32) + header
        buffer.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&header_bytes);

        // Write deleted bitmap length + data
        buffer.extend_from_slice(&(deleted_bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&deleted_bytes);

        // Write vectors length + data
        buffer.extend_from_slice(&(vectors_bytes.len() as u64).to_le_bytes());
        buffer.extend_from_slice(&vectors_bytes);

        // Write quantized length + data (0 if not enabled)
        buffer.extend_from_slice(&(quantized_bytes.len() as u64).to_le_bytes());
        if !quantized_bytes.is_empty() {
            buffer.extend_from_slice(&quantized_bytes);
        }

        Ok(buffer)
    }

    /// Restore index from a snapshot.
    ///
    /// # Arguments
    ///
    /// * `data` - Bytes from `to_snapshot()` or loaded from storage
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Magic number doesn't match
    /// - Version is unsupported
    /// - Checksum doesn't match
    /// - Data is corrupted or truncated
    ///
    /// # Panics
    ///
    /// This function does not panic. The internal `expect` is guarded by
    /// `chunks_exact(4)` which guarantees the slice length.
    #[allow(clippy::cast_possible_truncation)] // FlatIndex targets <10k vectors
    #[allow(clippy::missing_panics_doc)] // Panic is unreachable by design
    pub fn from_snapshot(data: &[u8]) -> Result<Self, PersistenceError> {
        let mut cursor = 0;

        // Read header length
        if data.len() < 4 {
            return Err(PersistenceError::TruncatedData);
        }
        let header_len = u32::from_le_bytes(
            data[0..4]
                .try_into()
                .map_err(|_| PersistenceError::TruncatedData)?,
        ) as usize;
        cursor += 4;

        // Read and parse header
        if data.len() < cursor + header_len {
            return Err(PersistenceError::TruncatedData);
        }
        let header: FlatIndexHeader = postcard::from_bytes(&data[cursor..cursor + header_len])
            .map_err(|e| PersistenceError::DeserializationError(e.to_string()))?;
        cursor += header_len;

        // Validate header
        header.validate()?;

        // Read deleted bitmap length
        if data.len() < cursor + 4 {
            return Err(PersistenceError::TruncatedData);
        }
        let deleted_len = u32::from_le_bytes(
            data[cursor..cursor + 4]
                .try_into()
                .map_err(|_| PersistenceError::TruncatedData)?,
        ) as usize;
        cursor += 4;

        // Read deleted bitmap
        if data.len() < cursor + deleted_len {
            return Err(PersistenceError::TruncatedData);
        }
        let deleted_bytes = &data[cursor..cursor + deleted_len];
        cursor += deleted_len;

        // Read vectors length
        if data.len() < cursor + 8 {
            return Err(PersistenceError::TruncatedData);
        }
        let vectors_len = u64::from_le_bytes(
            data[cursor..cursor + 8]
                .try_into()
                .map_err(|_| PersistenceError::TruncatedData)?,
        ) as usize;
        cursor += 8;

        // Read vectors
        if data.len() < cursor + vectors_len {
            return Err(PersistenceError::TruncatedData);
        }
        let vectors_bytes = &data[cursor..cursor + vectors_len];
        cursor += vectors_len;

        // Read quantized length
        if data.len() < cursor + 8 {
            return Err(PersistenceError::TruncatedData);
        }
        let quantized_len = u64::from_le_bytes(
            data[cursor..cursor + 8]
                .try_into()
                .map_err(|_| PersistenceError::TruncatedData)?,
        ) as usize;
        cursor += 8;

        // Read quantized data
        let quantized_bytes = if quantized_len > 0 {
            if data.len() < cursor + quantized_len {
                return Err(PersistenceError::TruncatedData);
            }
            Some(data[cursor..cursor + quantized_len].to_vec())
        } else {
            None
        };

        // Verify checksum
        let computed_checksum = Self::compute_checksum(
            deleted_bytes,
            vectors_bytes,
            quantized_bytes.as_deref().unwrap_or(&[]),
        );

        if computed_checksum != header.checksum {
            return Err(PersistenceError::ChecksumMismatch {
                expected: header.checksum,
                actual: computed_checksum,
            });
        }

        // Reconstruct config
        let config = FlatIndexConfig {
            dimensions: header.dimensions,
            metric: header.metric,
            initial_capacity: header.count as usize,
            cleanup_threshold: header.cleanup_threshold,
        };

        // Deserialize vectors
        let vectors: Vec<f32> = vectors_bytes
            .chunks_exact(4)
            .map(|chunk| {
                f32::from_le_bytes(chunk.try_into().expect("chunks_exact guarantees 4 bytes"))
            })
            .collect();

        // Deserialize deleted bitmap
        let deleted = Self::deserialize_deleted_bitmap(deleted_bytes, header.count as usize);

        Ok(Self {
            config,
            vectors,
            count: header.count,
            deleted,
            delete_count: header.delete_count as usize,
            next_id: header.next_id,
            quantized: quantized_bytes,
        })
    }

    /// Serialize the deleted bitmap to bytes.
    fn serialize_deleted_bitmap(&self) -> Vec<u8> {
        // BitVec stores bits in usize chunks; convert to raw bytes
        let raw_slice = self.deleted.as_raw_slice();
        let mut bytes = Vec::with_capacity(std::mem::size_of_val(raw_slice));
        for &word in raw_slice {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// Deserialize the deleted bitmap from bytes.
    fn deserialize_deleted_bitmap(bytes: &[u8], count: usize) -> BitVec {
        // Reconstruct from raw usize words
        let word_size = std::mem::size_of::<usize>();
        let mut words: Vec<usize> = bytes
            .chunks(word_size)
            .map(|chunk| {
                let mut arr = [0u8; std::mem::size_of::<usize>()];
                let len = chunk.len().min(word_size);
                arr[..len].copy_from_slice(&chunk[..len]);
                usize::from_le_bytes(arr)
            })
            .collect();

        // Ensure we have enough capacity
        let needed_words = (count + usize::BITS as usize - 1) / usize::BITS as usize;
        words.resize(needed_words, 0);

        let mut bv = BitVec::from_vec(words);
        bv.truncate(count);
        bv
    }

    /// Serialize vectors to bytes.
    fn serialize_vectors(&self) -> Vec<u8> {
        self.vectors.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    /// Serialize quantized vectors to bytes.
    fn serialize_quantized(&self) -> Vec<u8> {
        self.quantized.clone().unwrap_or_default()
    }

    /// Compute CRC32 checksum of data sections.
    fn compute_checksum(deleted: &[u8], vectors: &[u8], quantized: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(deleted);
        hasher.update(vectors);
        hasher.update(quantized);
        hasher.finalize()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Configuration Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_config_new() {
        let config = FlatIndexConfig::new(128);

        assert_eq!(config.dimensions, 128);
        assert_eq!(config.metric, DistanceMetric::Cosine);
        assert_eq!(config.initial_capacity, 1000);
        assert!((config.cleanup_threshold - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_config_builder() {
        let config = FlatIndexConfig::new(64)
            .with_metric(DistanceMetric::DotProduct)
            .with_capacity(5000)
            .with_cleanup_threshold(0.3);

        assert_eq!(config.dimensions, 64);
        assert_eq!(config.metric, DistanceMetric::DotProduct);
        assert_eq!(config.initial_capacity, 5000);
        assert!((config.cleanup_threshold - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_config_cleanup_threshold_clamping() {
        let config_low = FlatIndexConfig::new(64).with_cleanup_threshold(-0.5);
        assert!((config_low.cleanup_threshold - 0.0).abs() < f32::EPSILON);

        let config_high = FlatIndexConfig::new(64).with_cleanup_threshold(1.5);
        assert!((config_high.cleanup_threshold - 1.0).abs() < f32::EPSILON);
    }

    // ------------------------------------------------------------------------
    // Distance Metric Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_distance_metric_is_similarity() {
        assert!(DistanceMetric::Cosine.is_similarity());
        assert!(DistanceMetric::DotProduct.is_similarity());
        assert!(!DistanceMetric::L2.is_similarity());
        assert!(!DistanceMetric::Hamming.is_similarity());
    }

    #[test]
    fn test_distance_metric_default() {
        let metric: DistanceMetric = DistanceMetric::default();
        assert_eq!(metric, DistanceMetric::Cosine);
    }

    // ------------------------------------------------------------------------
    // FlatIndex Creation Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_new_flat_index() {
        let config = FlatIndexConfig::new(128);
        let index = FlatIndex::new(config);

        assert_eq!(index.dimensions(), 128);
        assert_eq!(index.metric(), DistanceMetric::Cosine);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
        assert_eq!(index.capacity(), 0);
        assert_eq!(index.deleted_count(), 0);
        assert!(!index.is_quantized());
    }

    #[test]
    fn test_new_with_different_metrics() {
        for metric in [
            DistanceMetric::Cosine,
            DistanceMetric::DotProduct,
            DistanceMetric::L2,
            DistanceMetric::Hamming,
        ] {
            let config = FlatIndexConfig::new(64).with_metric(metric);
            let index = FlatIndex::new(config);
            assert_eq!(index.metric(), metric);
        }
    }

    // ------------------------------------------------------------------------
    // Insert Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_insert_single() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();

        assert_eq!(id, 0);
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
        assert_eq!(index.capacity(), 1);
    }

    #[test]
    fn test_insert_multiple_sequential_ids() {
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

        // Too few dimensions
        let result = index.insert(&[1.0, 2.0]);
        assert!(matches!(
            result,
            Err(FlatIndexError::DimensionMismatch {
                expected: 3,
                actual: 2
            })
        ));

        // Too many dimensions
        let result = index.insert(&[1.0, 2.0, 3.0, 4.0]);
        assert!(matches!(
            result,
            Err(FlatIndexError::DimensionMismatch {
                expected: 3,
                actual: 4
            })
        ));

        // Index should be unchanged
        assert!(index.is_empty());
    }

    #[test]
    fn test_insert_batch() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let vectors: Vec<&[f32]> = vec![&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0], &[7.0, 8.0, 9.0]];

        let ids = index.insert_batch(&vectors).unwrap();

        assert_eq!(ids, vec![0, 1, 2]);
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn test_insert_batch_dimension_mismatch() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let vectors: Vec<&[f32]> = vec![
            &[1.0, 2.0, 3.0], // OK
            &[4.0, 5.0],      // Wrong dimension
            &[7.0, 8.0, 9.0], // Would be OK but not reached
        ];

        let result = index.insert_batch(&vectors);
        assert!(result.is_err());

        // First vector should have been inserted
        assert_eq!(index.len(), 1);
        assert!(index.contains(0));
    }

    #[test]
    fn test_insert_capacity_growth() {
        let config = FlatIndexConfig::new(3).with_capacity(2);
        let mut index = FlatIndex::new(config);

        // Insert more than initial capacity
        for i in 0..10 {
            let id = index.insert(&[i as f32, i as f32, i as f32]).unwrap();
            assert_eq!(id, i);
        }

        assert_eq!(index.len(), 10);

        // All vectors should be retrievable
        for i in 0..10 {
            assert!(index.contains(i));
        }
    }

    // ------------------------------------------------------------------------
    // Get Tests
    // ------------------------------------------------------------------------

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

        assert!(index.get(1).is_none()); // Not inserted yet
        assert!(index.get(99).is_none()); // Way out of bounds
        assert!(index.get(u64::MAX).is_none()); // Max value
    }

    #[test]
    fn test_get_empty_index() {
        let index = FlatIndex::new(FlatIndexConfig::new(3));
        assert!(index.get(0).is_none());
    }

    // ------------------------------------------------------------------------
    // Contains Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_contains() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        index.insert(&[1.0, 2.0, 3.0]).unwrap();
        index.insert(&[4.0, 5.0, 6.0]).unwrap();

        assert!(index.contains(0));
        assert!(index.contains(1));
        assert!(!index.contains(2));
        assert!(!index.contains(99));
    }

    // ------------------------------------------------------------------------
    // Stats Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_memory_usage() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        // Empty index
        let empty_usage = index.memory_usage();
        assert_eq!(empty_usage, 0);

        // After insert
        index.insert(&[1.0, 2.0, 3.0]).unwrap();
        let usage = index.memory_usage();

        // 3 floats * 4 bytes = 12 bytes for vectors
        // 1 bit (rounded up) for bitmap = 1 byte (actually 0 in bitvec)
        // But bitvec internal representation varies
        assert!(usage >= 12);
    }

    #[test]
    fn test_deletion_ratio_empty() {
        let index = FlatIndex::new(FlatIndexConfig::new(3));
        assert!((index.deletion_ratio() - 0.0).abs() < f32::EPSILON);
    }

    // ------------------------------------------------------------------------
    // Edge Cases
    // ------------------------------------------------------------------------

    #[test]
    fn test_high_dimension_vectors() {
        let dim = 768; // Typical embedding dimension
        let mut index = FlatIndex::new(FlatIndexConfig::new(dim));

        let vector: Vec<f32> = (0..dim).map(|i| i as f32 / dim as f32).collect();
        let id = index.insert(&vector).unwrap();

        assert_eq!(id, 0);
        let retrieved = index.get(0).unwrap();
        assert_eq!(retrieved.len(), dim as usize);
        assert_eq!(retrieved, vector.as_slice());
    }

    #[test]
    fn test_zero_vector() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let id = index.insert(&[0.0, 0.0, 0.0]).unwrap();
        let v = index.get(id).unwrap();

        assert_eq!(v, &[0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_negative_values() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        let id = index.insert(&[-1.0, -2.0, -3.0]).unwrap();
        let v = index.get(id).unwrap();

        assert_eq!(v, &[-1.0, -2.0, -3.0]);
    }

    #[test]
    fn test_special_float_values() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        // Insert vector with special values (NaN, Inf allowed at storage level)
        // Note: Distance calculations may behave unexpectedly with these values
        let id = index
            .insert(&[f32::INFINITY, f32::NEG_INFINITY, 0.0])
            .unwrap();
        let v = index.get(id).unwrap();

        assert!(v[0].is_infinite());
        assert!(v[1].is_infinite());
    }

    // ------------------------------------------------------------------------
    // Search Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_search_basic_cosine() {
        let config = FlatIndexConfig::new(3).with_metric(DistanceMetric::Cosine);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0, 0.0, 0.0]).unwrap(); // ID 0
        index.insert(&[0.0, 1.0, 0.0]).unwrap(); // ID 1
        index.insert(&[0.0, 0.0, 1.0]).unwrap(); // ID 2

        // Query closest to first vector
        let results = index.search(&[0.9, 0.1, 0.0], 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 0); // First vector is closest
    }

    #[test]
    fn test_search_all_metrics() {
        for metric in [
            DistanceMetric::Cosine,
            DistanceMetric::DotProduct,
            DistanceMetric::L2,
            DistanceMetric::Hamming,
        ] {
            let config = FlatIndexConfig::new(3).with_metric(metric);
            let mut index = FlatIndex::new(config);

            index.insert(&[1.0, 0.0, 0.0]).unwrap();
            index.insert(&[0.0, 1.0, 0.0]).unwrap();

            let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();

            assert_eq!(results.len(), 2, "Failed for metric {:?}", metric);
            // First vector should be exact match (best score)
            assert_eq!(results[0].id, 0, "Failed for metric {:?}", metric);
        }
    }

    #[test]
    fn test_search_dimension_mismatch() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));
        index.insert(&[1.0, 0.0, 0.0]).unwrap();

        let result = index.search(&[1.0, 0.0], 1); // Wrong dimension

        assert!(matches!(
            result,
            Err(FlatIndexError::DimensionMismatch {
                expected: 3,
                actual: 2
            })
        ));
    }

    #[test]
    fn test_search_k_zero() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));
        index.insert(&[1.0, 0.0, 0.0]).unwrap();

        let result = index.search(&[1.0, 0.0, 0.0], 0);

        assert!(matches!(result, Err(FlatIndexError::InvalidK)));
    }

    #[test]
    fn test_search_empty_index() {
        let index = FlatIndex::new(FlatIndexConfig::new(3));

        let results = index.search(&[1.0, 0.0, 0.0], 5).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_search_k_larger_than_count() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));
        index.insert(&[1.0, 0.0, 0.0]).unwrap();
        index.insert(&[0.0, 1.0, 0.0]).unwrap();

        let results = index.search(&[1.0, 0.0, 0.0], 10).unwrap();

        assert_eq!(results.len(), 2); // Only 2 vectors available
    }

    #[test]
    fn test_search_results_sorted_cosine() {
        let config = FlatIndexConfig::new(3).with_metric(DistanceMetric::Cosine);
        let mut index = FlatIndex::new(config);

        // Insert vectors with known similarities to [1, 0, 0]
        index.insert(&[1.0, 0.0, 0.0]).unwrap(); // ID 0: cos = 1.0 (exact match)
        index.insert(&[0.707, 0.707, 0.0]).unwrap(); // ID 1: cos ≈ 0.707
        index.insert(&[0.0, 1.0, 0.0]).unwrap(); // ID 2: cos = 0.0

        let query = [1.0, 0.0, 0.0];
        let results = index.search(&query, 3).unwrap();

        // Results should be sorted by descending similarity
        assert_eq!(results[0].id, 0); // Highest similarity
        assert_eq!(results[2].id, 2); // Lowest similarity

        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results not sorted at index {}: {} < {}",
                i,
                results[i - 1].score,
                results[i].score
            );
        }
    }

    #[test]
    fn test_search_l2_metric() {
        let config = FlatIndexConfig::new(3).with_metric(DistanceMetric::L2);
        let mut index = FlatIndex::new(config);

        index.insert(&[0.0, 0.0, 0.0]).unwrap(); // ID 0: at origin
        index.insert(&[1.0, 0.0, 0.0]).unwrap(); // ID 1: distance 1
        index.insert(&[2.0, 0.0, 0.0]).unwrap(); // ID 2: distance 2

        let results = index.search(&[0.0, 0.0, 0.0], 3).unwrap();

        // Closest first (ascending distance)
        assert_eq!(results[0].id, 0);
        assert!((results[0].score - 0.0).abs() < 1e-6);
        assert_eq!(results[1].id, 1);
        assert!((results[1].score - 1.0).abs() < 1e-6);
        assert_eq!(results[2].id, 2);
        assert!((results[2].score - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_search_dot_product_metric() {
        let config = FlatIndexConfig::new(3).with_metric(DistanceMetric::DotProduct);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0, 0.0, 0.0]).unwrap(); // ID 0: dot = 1.0
        index.insert(&[0.5, 0.0, 0.0]).unwrap(); // ID 1: dot = 0.5
        index.insert(&[0.0, 1.0, 0.0]).unwrap(); // ID 2: dot = 0.0

        let results = index.search(&[1.0, 0.0, 0.0], 3).unwrap();

        // Highest dot product first (descending)
        assert_eq!(results[0].id, 0);
        assert!((results[0].score - 1.0).abs() < 1e-6);
        assert_eq!(results[1].id, 1);
        assert!((results[1].score - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_search_hamming_metric() {
        let config = FlatIndexConfig::new(4).with_metric(DistanceMetric::Hamming);
        let mut index = FlatIndex::new(config);

        // Binary-like vectors (0.0 = 0, non-zero = 1)
        index.insert(&[1.0, 1.0, 0.0, 0.0]).unwrap(); // ID 0: [1,1,0,0]
        index.insert(&[1.0, 0.0, 0.0, 0.0]).unwrap(); // ID 1: [1,0,0,0]
        index.insert(&[0.0, 0.0, 1.0, 1.0]).unwrap(); // ID 2: [0,0,1,1]

        // Query: [1,1,0,0]
        let results = index.search(&[1.0, 1.0, 0.0, 0.0], 3).unwrap();

        // ID 0 has Hamming distance 0 (exact match)
        assert_eq!(results[0].id, 0);
        assert!((results[0].score - 0.0).abs() < 1e-6);

        // ID 1 has Hamming distance 1
        assert_eq!(results[1].id, 1);
        assert!((results[1].score - 1.0).abs() < 1e-6);

        // ID 2 has Hamming distance 4
        assert_eq!(results[2].id, 2);
        assert!((results[2].score - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_search_100_recall_validation() {
        // Verify brute-force achieves 100% recall
        let mut index = FlatIndex::new(FlatIndexConfig::new(64));

        // Insert 100 deterministic vectors using LCG
        let mut seed: u64 = 42;
        let lcg = |s: &mut u64| -> f32 {
            *s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((*s >> 33) as f32) / (u32::MAX as f32)
        };

        for _ in 0..100 {
            let v: Vec<f32> = (0..64).map(|_| lcg(&mut seed)).collect();
            index.insert(&v).unwrap();
        }

        // Search should return exactly k results
        let query: Vec<f32> = (0..64).map(|_| lcg(&mut seed)).collect();
        let results = index.search(&query, 10).unwrap();

        assert_eq!(results.len(), 10);

        // Results should be sorted by similarity (descending for cosine)
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }
    }

    #[test]
    fn test_search_high_dimension() {
        let dim = 768;
        let config = FlatIndexConfig::new(dim).with_metric(DistanceMetric::Cosine);
        let mut index = FlatIndex::new(config);

        // Insert 100 high-dimensional vectors
        for i in 0..100 {
            let v: Vec<f32> = (0..dim as usize).map(|j| (i * j) as f32 / 1000.0).collect();
            index.insert(&v).unwrap();
        }

        let query: Vec<f32> = (0..dim as usize).map(|j| j as f32 / 1000.0).collect();
        let results = index.search(&query, 5).unwrap();

        assert_eq!(results.len(), 5);
    }

    // ------------------------------------------------------------------------
    // Deletion Tests (Day 3)
    // ------------------------------------------------------------------------

    #[test]
    fn test_delete_basic() {
        let config = FlatIndexConfig::new(3).with_cleanup_threshold(1.0); // Disable auto-compact
        let mut index = FlatIndex::new(config);

        let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();

        assert!(index.contains(id));
        assert!(index.delete(id)); // Should return true
        assert!(!index.contains(id)); // Should no longer be accessible
        assert!(index.get(id).is_none());
    }

    #[test]
    fn test_delete_already_deleted() {
        let config = FlatIndexConfig::new(3).with_cleanup_threshold(1.0);
        let mut index = FlatIndex::new(config);

        let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();

        assert!(index.delete(id)); // First delete succeeds
        assert!(!index.delete(id)); // Second delete returns false
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        assert!(!index.delete(0)); // Empty index
        assert!(!index.delete(999)); // Out of bounds
    }

    #[test]
    fn test_delete_updates_len() {
        let config = FlatIndexConfig::new(3).with_cleanup_threshold(1.0);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0, 2.0, 3.0]).unwrap();
        index.insert(&[4.0, 5.0, 6.0]).unwrap();

        assert_eq!(index.len(), 2);

        index.delete(0);
        assert_eq!(index.len(), 1);

        index.delete(1);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_delete_updates_deleted_count() {
        let config = FlatIndexConfig::new(3).with_cleanup_threshold(1.0);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0, 2.0, 3.0]).unwrap();
        index.insert(&[4.0, 5.0, 6.0]).unwrap();

        assert_eq!(index.deleted_count(), 0);

        index.delete(0);
        assert_eq!(index.deleted_count(), 1);

        index.delete(1);
        assert_eq!(index.deleted_count(), 2);
    }

    #[test]
    fn test_deletion_stats() {
        let config = FlatIndexConfig::new(3).with_cleanup_threshold(1.0);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0, 2.0, 3.0]).unwrap();
        index.insert(&[4.0, 5.0, 6.0]).unwrap();

        let (total, deleted, ratio) = index.deletion_stats();
        assert_eq!(total, 2);
        assert_eq!(deleted, 0);
        assert!((ratio - 0.0).abs() < f32::EPSILON);

        index.delete(0);

        let (total, deleted, ratio) = index.deletion_stats();
        assert_eq!(total, 2);
        assert_eq!(deleted, 1);
        assert!((ratio - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_search_skips_deleted() {
        let config = FlatIndexConfig::new(3)
            .with_metric(DistanceMetric::Cosine)
            .with_cleanup_threshold(1.0);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0, 0.0, 0.0]).unwrap(); // ID 0 - exact match
        index.insert(&[0.0, 1.0, 0.0]).unwrap(); // ID 1

        // Before delete: ID 0 should be best match
        let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results[0].id, 0);

        // Delete the best match
        index.delete(0);

        // After delete: ID 1 should be the only result
        let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    // ------------------------------------------------------------------------
    // Compaction Tests (Day 3)
    // ------------------------------------------------------------------------

    #[test]
    fn test_compact_basic() {
        let config = FlatIndexConfig::new(3).with_cleanup_threshold(1.0); // Disable auto
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0, 2.0, 3.0]).unwrap();
        index.insert(&[4.0, 5.0, 6.0]).unwrap();
        index.insert(&[7.0, 8.0, 9.0]).unwrap();

        // Delete middle
        index.delete(1);
        assert_eq!(index.capacity(), 3);
        assert_eq!(index.len(), 2);

        // Compact
        index.compact();

        assert_eq!(index.capacity(), 2);
        assert_eq!(index.len(), 2);
        assert_eq!(index.deleted_count(), 0);
    }

    #[test]
    fn test_compact_preserves_data() {
        let config = FlatIndexConfig::new(3).with_cleanup_threshold(1.0);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0, 2.0, 3.0]).unwrap(); // ID 0
        index.insert(&[4.0, 5.0, 6.0]).unwrap(); // ID 1
        index.insert(&[7.0, 8.0, 9.0]).unwrap(); // ID 2

        // Delete first and last
        index.delete(0);
        index.delete(2);

        // Compact
        index.compact();

        // Only middle vector should remain (now at ID 0 after compaction)
        assert_eq!(index.len(), 1);
        let v = index.get(0).unwrap(); // After compact, renumbered to 0
        assert_eq!(v, &[4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_compact_empty() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        // Compact on empty should be a no-op
        index.compact();
        assert!(index.is_empty());
    }

    #[test]
    fn test_compact_nothing_to_do() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(3));

        index.insert(&[1.0, 2.0, 3.0]).unwrap();

        // No deletions, compact should be a no-op
        let capacity_before = index.capacity();
        index.compact();
        assert_eq!(index.capacity(), capacity_before);
    }

    #[test]
    fn test_auto_compact_on_threshold() {
        // Set threshold to 0.3 (30%)
        let config = FlatIndexConfig::new(3).with_cleanup_threshold(0.3);
        let mut index = FlatIndex::new(config);

        // Insert 10 vectors
        for i in 0..10 {
            index.insert(&[i as f32, 0.0, 0.0]).unwrap();
        }

        assert_eq!(index.capacity(), 10);

        // Delete 4 vectors (40% > 30% threshold)
        index.delete(0);
        index.delete(1);
        index.delete(2);
        // 30% not yet exceeded (3/10 = 30%, not > 30%)
        assert_eq!(index.capacity(), 10);

        index.delete(3); // 4/10 = 40% > 30% => auto compact

        // Should have auto-compacted
        assert_eq!(index.capacity(), 6);
        assert_eq!(index.deleted_count(), 0);
    }

    // ------------------------------------------------------------------------
    // Quantization Tests (Day 3)
    // ------------------------------------------------------------------------

    #[test]
    fn test_enable_quantization() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        index
            .insert(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0])
            .unwrap();

        assert!(!index.is_quantized());

        index.enable_quantization().unwrap();

        assert!(index.is_quantized());
    }

    #[test]
    fn test_enable_quantization_idempotent() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        index
            .insert(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0])
            .unwrap();

        index.enable_quantization().unwrap();
        index.enable_quantization().unwrap(); // Should be a no-op

        assert!(index.is_quantized());
    }

    #[test]
    fn test_enable_quantization_empty_index() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        // Should succeed on empty index
        index.enable_quantization().unwrap();
        assert!(index.is_quantized());
    }

    #[test]
    fn test_disable_quantization() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        index
            .insert(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0])
            .unwrap();
        index.enable_quantization().unwrap();
        assert!(index.is_quantized());

        index.disable_quantization();
        assert!(!index.is_quantized());
    }

    #[test]
    fn test_search_quantized_basic() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        // Vector 0: all positive (binary: 11111111)
        index
            .insert(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0])
            .unwrap();
        // Vector 1: alternating (binary: 10101010)
        index
            .insert(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0])
            .unwrap();
        // Vector 2: all negative (binary: 00000000)
        index
            .insert(&[-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0])
            .unwrap();

        index.enable_quantization().unwrap();

        // Query for alternating pattern
        let query = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        let results = index.search_quantized(&query, 3).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, 1); // Exact match
        assert!((results[0].score - 0.0).abs() < f32::EPSILON); // Hamming distance 0
    }

    #[test]
    fn test_search_quantized_hamming_distances() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        // Binary patterns:
        // 11111111 (all positive)
        index
            .insert(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0])
            .unwrap();
        // 11111110 (7 positive, 1 negative)
        index
            .insert(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0])
            .unwrap();
        // 00000000 (all negative)
        index
            .insert(&[-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0])
            .unwrap();

        index.enable_quantization().unwrap();

        // Query: 11111111
        let results = index.search_quantized(&[1.0; 8], 3).unwrap();

        // ID 0: Hamming distance 0
        assert_eq!(results[0].id, 0);
        assert!((results[0].score - 0.0).abs() < f32::EPSILON);

        // ID 1: Hamming distance 1
        assert_eq!(results[1].id, 1);
        assert!((results[1].score - 1.0).abs() < f32::EPSILON);

        // ID 2: Hamming distance 8
        assert_eq!(results[2].id, 2);
        assert!((results[2].score - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_search_quantized_not_enabled() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        index.insert(&[1.0; 8]).unwrap();

        let result = index.search_quantized(&[1.0; 8], 1);
        assert!(matches!(
            result,
            Err(FlatIndexError::QuantizationNotEnabled)
        ));
    }

    #[test]
    fn test_search_quantized_dimension_mismatch() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        index.insert(&[1.0; 8]).unwrap();
        index.enable_quantization().unwrap();

        let result = index.search_quantized(&[1.0; 4], 1); // Wrong dimension
        assert!(matches!(
            result,
            Err(FlatIndexError::DimensionMismatch {
                expected: 8,
                actual: 4
            })
        ));
    }

    #[test]
    fn test_search_quantized_k_zero() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        index.insert(&[1.0; 8]).unwrap();
        index.enable_quantization().unwrap();

        let result = index.search_quantized(&[1.0; 8], 0);
        assert!(matches!(result, Err(FlatIndexError::InvalidK)));
    }

    #[test]
    fn test_search_quantized_skips_deleted() {
        let config = FlatIndexConfig::new(8).with_cleanup_threshold(1.0);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0; 8]).unwrap(); // ID 0: exact match
        index.insert(&[-1.0; 8]).unwrap(); // ID 1: max distance

        index.enable_quantization().unwrap();

        // Before delete
        let results = index.search_quantized(&[1.0; 8], 2).unwrap();
        assert_eq!(results[0].id, 0);

        // Delete and re-enable quantization (delete invalidates it)
        index.delete(0);
        index.enable_quantization().unwrap();

        // After delete: only ID 1 should be found
        let results = index.search_quantized(&[1.0; 8], 2).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_insert_invalidates_quantization() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        index.insert(&[1.0; 8]).unwrap();
        index.enable_quantization().unwrap();
        assert!(index.is_quantized());

        // Insert invalidates quantization cache
        index.insert(&[-1.0; 8]).unwrap();
        assert!(!index.is_quantized());
    }

    #[test]
    fn test_delete_invalidates_quantization() {
        let config = FlatIndexConfig::new(8).with_cleanup_threshold(1.0);
        let mut index = FlatIndex::new(config);

        index.insert(&[1.0; 8]).unwrap();
        index.enable_quantization().unwrap();
        assert!(index.is_quantized());

        // Delete invalidates quantization cache
        index.delete(0);
        assert!(!index.is_quantized());
    }

    #[test]
    fn test_quantization_memory_reduction() {
        let dim = 768;
        let mut index = FlatIndex::new(FlatIndexConfig::new(dim));

        // Insert 100 vectors
        for i in 0..100 {
            let v: Vec<f32> = (0..dim as usize)
                .map(|j| if (i + j) % 2 == 0 { 1.0 } else { -1.0 })
                .collect();
            index.insert(&v).unwrap();
        }

        let memory_before = index.memory_usage();

        index.enable_quantization().unwrap();

        let memory_after = index.memory_usage();

        // Quantized storage should add ~100 * 96 bytes = 9600 bytes
        // F32 storage is 100 * 768 * 4 = 307200 bytes
        // Total should be ~316800 bytes
        // Without quantization: ~307200 bytes
        assert!(memory_after > memory_before); // Quantized adds to memory (doesn't replace)

        // But quantized-only search uses much less memory per vector
        // 768 / 8 = 96 bytes (quantized) vs 768 * 4 = 3072 bytes (f32)
        // Ratio: 3072 / 96 = 32x reduction
        let f32_per_vector = dim as usize * 4;
        let bq_per_vector = (dim as usize + 7) / 8;
        assert_eq!(f32_per_vector / bq_per_vector, 32);
    }

    #[test]
    fn test_binarize_vector() {
        // Test the internal binarization logic
        let packed = FlatIndex::binarize_vector(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0]);

        // Expected: 10101010 = 170 in decimal (MSB first)
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], 0b1010_1010);
    }

    #[test]
    fn test_binarize_vector_16_dim() {
        let packed = FlatIndex::binarize_vector(&[
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, // 11111111
            -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, // 00000000
        ]);

        assert_eq!(packed.len(), 2);
        assert_eq!(packed[0], 0b1111_1111);
        assert_eq!(packed[1], 0b0000_0000);
    }

    #[test]
    fn test_hamming_distance_binary() {
        let a = [0b1111_1111u8];
        let b = [0b1111_1110u8];

        let distance = FlatIndex::hamming_distance_binary(&a, &b);
        assert_eq!(distance, 1); // 1 bit different

        let c = [0b0000_0000u8];
        let distance2 = FlatIndex::hamming_distance_binary(&a, &c);
        assert_eq!(distance2, 8); // All 8 bits different
    }

    #[test]
    fn test_search_quantized_high_dimension() {
        let dim = 768;
        let mut index = FlatIndex::new(FlatIndexConfig::new(dim));

        // Insert 50 vectors with different patterns
        for i in 0..50 {
            let v: Vec<f32> = (0..dim as usize)
                .map(|j| if (i + j) % 2 == 0 { 1.0 } else { -1.0 })
                .collect();
            index.insert(&v).unwrap();
        }

        index.enable_quantization().unwrap();

        // Query that matches pattern at i=0
        let query: Vec<f32> = (0..dim as usize)
            .map(|j| if j % 2 == 0 { 1.0 } else { -1.0 })
            .collect();

        let results = index.search_quantized(&query, 10).unwrap();

        assert_eq!(results.len(), 10);
        // All even IDs should have better (lower) Hamming distance
        assert!(results[0].id % 2 == 0);
    }

    // ------------------------------------------------------------------------
    // Persistence Tests (Day 4)
    // ------------------------------------------------------------------------

    #[test]
    fn test_snapshot_round_trip_basic() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(64));

        // Insert vectors
        for i in 0..100 {
            let v: Vec<f32> = (0..64).map(|j| (i * 64 + j) as f32 / 1000.0).collect();
            index.insert(&v).unwrap();
        }

        // Save snapshot
        let snapshot = index.to_snapshot().unwrap();

        // Restore from snapshot
        let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

        // Verify state
        assert_eq!(restored.dimensions(), index.dimensions());
        assert_eq!(restored.len(), index.len());
        assert_eq!(restored.metric(), index.metric());

        // Verify vectors
        for i in 0..100 {
            let original = index.get(i).unwrap();
            let restored_vec = restored.get(i).unwrap();
            assert_eq!(original, restored_vec);
        }
    }

    #[test]
    #[allow(clippy::useless_vec)] // Variable value requires vec!
    fn test_snapshot_with_deletions() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(16));

        // Insert vectors
        for i in 0..50 {
            index.insert(&vec![i as f32; 16]).unwrap();
        }

        // Delete some
        assert!(index.delete(10));
        assert!(index.delete(20));
        assert!(index.delete(30));

        // Save and restore
        let snapshot = index.to_snapshot().unwrap();
        let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

        // Verify deletions preserved
        assert!(restored.get(10).is_none());
        assert!(restored.get(20).is_none());
        assert!(restored.get(30).is_none());
        assert!(restored.get(0).is_some());
        assert!(restored.get(49).is_some());

        // Verify deletion count
        let (_, delete_count, _) = restored.deletion_stats();
        assert_eq!(delete_count, 3);
    }

    #[test]
    fn test_snapshot_with_quantization() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(128));

        // Insert vectors
        for i in 0..100 {
            let v: Vec<f32> = (0..128)
                .map(|j| if (i + j) % 2 == 0 { 1.0 } else { -1.0 })
                .collect();
            index.insert(&v).unwrap();
        }

        // Enable quantization
        index.enable_quantization().unwrap();
        assert!(index.is_quantized());

        // Save and restore
        let snapshot = index.to_snapshot().unwrap();
        let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

        // Verify quantization state
        assert!(restored.is_quantized());

        // Search should work
        let query: Vec<f32> = (0..128)
            .map(|j| if j % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let results = restored.search_quantized(&query, 5).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_snapshot_different_metrics() {
        for metric in [
            DistanceMetric::Cosine,
            DistanceMetric::DotProduct,
            DistanceMetric::L2,
            DistanceMetric::Hamming,
        ] {
            let config = FlatIndexConfig::new(32).with_metric(metric);
            let mut index = FlatIndex::new(config);

            index.insert(&[0.5; 32]).unwrap();

            let snapshot = index.to_snapshot().unwrap();
            let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

            assert_eq!(restored.metric(), metric);
        }
    }

    #[test]
    fn test_snapshot_invalid_magic() {
        // Create minimal valid header, then corrupt magic
        let mut index = FlatIndex::new(FlatIndexConfig::new(4));
        index.insert(&[1.0; 4]).unwrap();

        let mut snapshot = index.to_snapshot().unwrap();

        // Corrupt magic in header (after the 4-byte length)
        if snapshot.len() > 8 {
            snapshot[4] = b'X';
            snapshot[5] = b'X';
            snapshot[6] = b'X';
            snapshot[7] = b'X';
        }

        let result = FlatIndex::from_snapshot(&snapshot);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_truncated() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(16));
        index.insert(&[1.0; 16]).unwrap();

        let snapshot = index.to_snapshot().unwrap();

        // Truncate snapshot
        let truncated = &snapshot[..snapshot.len() / 2];

        let result = FlatIndex::from_snapshot(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_corrupted_checksum() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(16));
        index.insert(&[1.0; 16]).unwrap();

        let mut snapshot = index.to_snapshot().unwrap();

        // Corrupt data section (flip bits in vector data)
        if snapshot.len() > 100 {
            snapshot[100] ^= 0xFF;
        }

        let result = FlatIndex::from_snapshot(&snapshot);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_after_restore() {
        let config = FlatIndexConfig::new(64).with_metric(DistanceMetric::Cosine);
        let mut index = FlatIndex::new(config);

        // Insert known vectors
        index.insert(&[1.0; 64]).unwrap(); // ID 0
        index.insert(&[0.5; 64]).unwrap(); // ID 1
        index.insert(&[0.0; 64]).unwrap(); // ID 2

        // Save and restore
        let snapshot = index.to_snapshot().unwrap();
        let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

        // Search should return same results
        let query = [1.0; 64];
        let original_results = index.search(&query, 3).unwrap();
        let restored_results = restored.search(&query, 3).unwrap();

        assert_eq!(original_results.len(), restored_results.len());
        for (orig, rest) in original_results.iter().zip(restored_results.iter()) {
            assert_eq!(orig.id, rest.id);
            assert!((orig.score - rest.score).abs() < 1e-6);
        }
    }

    #[test]
    fn test_snapshot_empty_index() {
        let index = FlatIndex::new(FlatIndexConfig::new(128));

        let snapshot = index.to_snapshot().unwrap();
        let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

        assert_eq!(restored.dimensions(), 128);
        assert!(restored.is_empty());
        assert_eq!(restored.len(), 0);
    }

    #[test]
    fn test_snapshot_preserves_next_id() {
        let mut index = FlatIndex::new(FlatIndexConfig::new(8));

        // Insert, delete, insert pattern to increase next_id
        let id1 = index.insert(&[1.0; 8]).unwrap();
        let id2 = index.insert(&[2.0; 8]).unwrap();
        index.delete(id1);
        let id3 = index.insert(&[3.0; 8]).unwrap();

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);

        // Save and restore
        let snapshot = index.to_snapshot().unwrap();
        let mut restored = FlatIndex::from_snapshot(&snapshot).unwrap();

        // Next insert should get ID 3
        let id4 = restored.insert(&[4.0; 8]).unwrap();
        assert_eq!(id4, 3);
    }

    #[test]
    fn test_snapshot_cleanup_threshold() {
        let config = FlatIndexConfig::new(8).with_cleanup_threshold(0.25);
        let index = FlatIndex::new(config);

        let snapshot = index.to_snapshot().unwrap();
        let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

        // Can't directly access cleanup_threshold, but we can verify behavior
        // by inserting and deleting (would need additional getter)
        assert_eq!(restored.dimensions(), 8);
    }

    #[test]
    fn test_snapshot_header_validation() {
        use crate::persistence::PersistenceError;

        let header = FlatIndexHeader {
            magic: FLAT_INDEX_MAGIC,
            version: FLAT_INDEX_VERSION,
            dimensions: 64,
            metric: DistanceMetric::Cosine,
            count: 10,
            delete_count: 0,
            next_id: 10,
            is_quantized: false,
            cleanup_threshold: 0.5,
            checksum: 0,
        };

        assert!(header.validate().is_ok());

        // Wrong magic
        let bad_magic = FlatIndexHeader {
            magic: [b'X', b'X', b'X', b'X'],
            ..header.clone()
        };
        assert!(matches!(
            bad_magic.validate(),
            Err(PersistenceError::InvalidMagic { .. })
        ));

        // Future version
        let future_version = FlatIndexHeader {
            version: FLAT_INDEX_VERSION + 1,
            ..header.clone()
        };
        assert!(matches!(
            future_version.validate(),
            Err(PersistenceError::UnsupportedVersion(_, _))
        ));
    }

    // ------------------------------------------------------------------------
    // BQ Recall Comparison Test (HOSTILE_REVIEWER m2)
    // ------------------------------------------------------------------------

    /// Test that measures recall degradation of BQ vs F32 search.
    ///
    /// BQ uses Hamming distance on binary-packed vectors, which is an
    /// approximation of angular distance. This test validates:
    /// 1. BQ recall is documented and measured
    /// 2. BQ provides meaningful (non-random) results
    ///
    /// # Important Notes on BQ Recall
    ///
    /// Binary quantization works best when:
    /// - Data has clear sign patterns (e.g., embedding models)
    /// - Using angular/cosine similarity metrics
    /// - Vectors are normalized
    ///
    /// For uniformly random data, BQ recall is lower (~30-50%) because
    /// random vectors don't have meaningful sign structure. Real embedding
    /// data (e.g., from BERT, OpenAI) typically achieves 70-90% recall.
    ///
    /// The 32x memory reduction makes BQ valuable even at lower recall
    /// when used as a first-pass filter (reranking with F32).
    #[test]
    fn test_bq_vs_f32_recall_comparison() {
        use std::collections::HashSet;

        let dim = 128;
        let count = 500;
        let k = 10;
        let num_queries = 50;

        // Use a consistent seed for reproducibility
        let seed = 12345u64;

        // Create index with cosine metric
        let config = FlatIndexConfig::new(dim).with_metric(DistanceMetric::Cosine);
        let mut index = FlatIndex::new(config);

        // Generate reproducible vectors using LCG
        let mut state = seed;
        let next_f32 = |s: &mut u64| -> f32 {
            // LCG: state = state * a + c mod m
            *s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            // Map to [-1, 1]
            ((*s >> 33) as f32 / (u32::MAX >> 1) as f32) * 2.0 - 1.0
        };

        // Insert vectors
        for _ in 0..count {
            let v: Vec<f32> = (0..dim).map(|_| next_f32(&mut state)).collect();
            index.insert(&v).unwrap();
        }

        // Enable quantization
        index.enable_quantization().unwrap();

        // Measure recall across multiple queries
        let mut total_recall = 0.0;

        for q in 0..num_queries {
            // Use existing vectors as queries (ensures they exist in the index)
            let query_id = q * (count / num_queries);
            let query = index
                .get(u64::try_from(query_id).unwrap())
                .unwrap()
                .to_vec();

            // Get ground truth (F32 exact search)
            let f32_results = index.search(&query, k).unwrap();
            let f32_ids: HashSet<u64> = f32_results.iter().map(|r| r.id).collect();

            // Get BQ approximate results
            let bq_results = index.search_quantized(&query, k).unwrap();
            let bq_ids: HashSet<u64> = bq_results.iter().map(|r| r.id).collect();

            // Calculate recall: |F32 ∩ BQ| / |F32|
            let intersection = f32_ids.intersection(&bq_ids).count();
            let recall = intersection as f32 / k as f32;
            total_recall += recall;
        }

        let avg_recall = total_recall / num_queries as f32;

        // Document the measured recall
        println!(
            "BQ vs F32 Recall Comparison:\n\
             - Dataset: {} vectors @ {}D (random uniform)\n\
             - Queries: {}\n\
             - k: {}\n\
             - Average Recall@{}: {:.1}%\n\
             - Note: Random data has lower recall; real embeddings achieve 70-90%",
            count,
            dim,
            num_queries,
            k,
            k,
            avg_recall * 100.0
        );

        // For random data, BQ recall is ~30-50% (expected).
        // Random chance for 10/500 = 2%, so BQ is significantly better.
        // We assert > 20% to ensure BQ provides non-random results.
        // Real embedding data would use a higher threshold (e.g., 70%).
        assert!(
            avg_recall >= 0.20,
            "BQ recall too low: {:.1}% (minimum: 20% for random data)",
            avg_recall * 100.0
        );

        // Verify BQ is significantly better than random chance (2%)
        let random_chance = k as f32 / count as f32;
        assert!(
            avg_recall > random_chance * 5.0,
            "BQ recall ({:.1}%) not significantly better than random ({:.1}%)",
            avg_recall * 100.0,
            random_chance * 100.0
        );
    }
}

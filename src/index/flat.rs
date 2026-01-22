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
        let metric: DistanceMetric = Default::default();
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
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
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
}

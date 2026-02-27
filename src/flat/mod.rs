//! Flat (brute-force) index for binary vectors.
//!
//! This module provides a simple flat index optimized for binary vectors.
//! Unlike HNSW, it uses O(1) insert and O(n) search, which is faster for
//! small-to-medium datasets (< 100K vectors) due to the extremely fast
//! SIMD Hamming distance calculation.
//!
//! # Performance Characteristics
//!
//! | Operation | Complexity | Time (10K vectors) |
//! |-----------|------------|-------------------|
//! | Insert    | O(1)       | ~1 μs             |
//! | Search    | O(n)       | ~1ms (SIMD)       |
//!
//! # When to Use
//!
//! - Insert-heavy workloads (semantic caching)
//! - Datasets < 100K vectors
//! - When 100% recall (exact search) is required
//! - When insert latency is critical

use crate::hnsw::VectorId;
use crate::metric::{Hamming, Metric};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when using `BinaryFlatIndex`.
#[derive(Debug, Clone, Error)]
pub enum BinaryFlatIndexError {
    /// Dimensions must be divisible by 8.
    #[error("dimensions must be divisible by 8, got {0}")]
    InvalidDimensions(usize),

    /// Vector length doesn't match expected bytes.
    #[error("vector length {actual} doesn't match expected {expected}")]
    DimensionMismatch {
        /// Expected number of bytes.
        expected: usize,
        /// Actual number of bytes provided.
        actual: usize,
    },

    /// Capacity overflow when allocating storage.
    #[error("capacity overflow: {0} * {1} exceeds usize::MAX")]
    CapacityOverflow(usize, usize),
}

/// Search result from binary flat index.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BinaryFlatSearchResult {
    /// Vector ID.
    pub id: VectorId,
    /// Hamming distance (lower is more similar).
    pub distance: f32,
}

/// Threshold ratio for choosing partial sort vs full sort in search.
///
/// When `k < count / PARTIAL_SORT_THRESHOLD`, we use `select_nth_unstable`
/// followed by sorting only the top k elements. This is more efficient for
/// small k values relative to the total count.
///
/// Value of 10 means: use partial sort when k is less than 10% of total vectors.
const PARTIAL_SORT_THRESHOLD: usize = 10;

/// Size of serialization header in bytes (dimensions u32 + count u32).
const SERIALIZATION_HEADER_SIZE: usize = 8;

/// A flat (brute-force) index for binary vectors.
///
/// Stores vectors in a contiguous array for cache-friendly linear scan.
/// Insert is O(1), search is O(n) with SIMD-accelerated Hamming distance.
///
/// # ID Convention
///
/// `BinaryFlatIndex` follows the `VectorStorage` convention and uses
/// **1-based IDs** where `0` is reserved as `INVALID_ID`. The first inserted
/// vector receives `VectorId(1)`, the second `VectorId(2)`, and so on.
/// This differs from [`FlatIndex`](crate::index::FlatIndex) which uses
/// 0-based direct indexing. The 1-based convention allows `VectorId(0)` to
/// serve as a sentinel value in graph structures (e.g., HNSW neighbor lists).
#[derive(Debug, Serialize, Deserialize)]
pub struct BinaryFlatIndex {
    /// Contiguous storage for all vectors.
    vectors: Vec<u8>,
    /// Number of bits per vector.
    dimensions: usize,
    /// Number of bytes per vector (dimensions / 8).
    bytes_per_vector: usize,
    /// Number of vectors stored.
    count: usize,
}

impl BinaryFlatIndex {
    /// Create a new binary flat index.
    ///
    /// # Arguments
    ///
    /// * `dimensions` - Number of bits per vector (must be divisible by 8)
    ///
    /// # Errors
    ///
    /// Returns [`BinaryFlatIndexError::InvalidDimensions`] if dimensions is not divisible by 8.
    #[must_use = "constructors return a Result that should be used"]
    pub fn new(dimensions: usize) -> Result<Self, BinaryFlatIndexError> {
        if dimensions % 8 != 0 {
            return Err(BinaryFlatIndexError::InvalidDimensions(dimensions));
        }
        Ok(Self {
            vectors: Vec::new(),
            dimensions,
            bytes_per_vector: dimensions / 8,
            count: 0,
        })
    }

    /// Create a new binary flat index with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `dimensions` - Number of bits per vector (must be divisible by 8)
    /// * `capacity` - Number of vectors to pre-allocate space for
    ///
    /// # Errors
    ///
    /// Returns [`BinaryFlatIndexError::InvalidDimensions`] if dimensions is not divisible by 8.
    /// Returns [`BinaryFlatIndexError::CapacityOverflow`] if capacity * bytes_per_vector overflows.
    #[must_use = "constructors return a Result that should be used"]
    pub fn with_capacity(dimensions: usize, capacity: usize) -> Result<Self, BinaryFlatIndexError> {
        if dimensions % 8 != 0 {
            return Err(BinaryFlatIndexError::InvalidDimensions(dimensions));
        }
        let bytes_per_vector = dimensions / 8;
        let total_bytes = capacity.checked_mul(bytes_per_vector).ok_or(
            BinaryFlatIndexError::CapacityOverflow(capacity, bytes_per_vector),
        )?;
        Ok(Self {
            vectors: Vec::with_capacity(total_bytes),
            dimensions,
            bytes_per_vector,
            count: 0,
        })
    }

    /// Insert a binary vector into the index.
    ///
    /// # Arguments
    ///
    /// * `vector` - Binary vector as packed bytes
    ///
    /// # Returns
    ///
    /// The ID of the inserted vector.
    ///
    /// # Errors
    ///
    /// Returns [`BinaryFlatIndexError::DimensionMismatch`] if vector length doesn't match.
    #[inline]
    #[must_use = "insert returns the assigned VectorId"]
    pub fn insert(&mut self, vector: &[u8]) -> Result<VectorId, BinaryFlatIndexError> {
        if vector.len() != self.bytes_per_vector {
            return Err(BinaryFlatIndexError::DimensionMismatch {
                expected: self.bytes_per_vector,
                actual: vector.len(),
            });
        }

        self.vectors.extend_from_slice(vector);
        self.count += 1;
        // Start IDs at 1 to match EdgeVec/VectorStorage convention (0 is reserved sentinel)
        Ok(VectorId(self.count as u64))
    }

    /// Search for the k nearest neighbors to a query vector.
    ///
    /// Uses SIMD-accelerated Hamming distance for fast linear scan.
    ///
    /// # Arguments
    ///
    /// * `query` - Query vector as packed bytes
    /// * `k` - Number of nearest neighbors to return
    ///
    /// # Returns
    ///
    /// Vector of search results sorted by distance (ascending).
    ///
    /// # Errors
    ///
    /// Returns [`BinaryFlatIndexError::DimensionMismatch`] if query length doesn't match.
    #[must_use = "search returns the nearest neighbors"]
    pub fn search(
        &self,
        query: &[u8],
        k: usize,
    ) -> Result<Vec<BinaryFlatSearchResult>, BinaryFlatIndexError> {
        if query.len() != self.bytes_per_vector {
            return Err(BinaryFlatIndexError::DimensionMismatch {
                expected: self.bytes_per_vector,
                actual: query.len(),
            });
        }

        if self.count == 0 || k == 0 {
            return Ok(Vec::new());
        }

        let k = k.min(self.count);

        // For small k, use a simple approach: compute all distances, partial sort
        // For larger datasets, could use a heap for O(n log k) instead of O(n log n)
        let mut results: Vec<(VectorId, f32)> = Vec::with_capacity(self.count);

        for i in 0..self.count {
            let start = i * self.bytes_per_vector;
            let end = start + self.bytes_per_vector;
            let stored = &self.vectors[start..end];
            let dist = Hamming::distance(query, stored);
            // IDs are 1-based (i+1) to match EdgeVec convention
            results.push((VectorId((i + 1) as u64), dist));
        }

        // Partial sort to get top k (more efficient than full sort for small k)
        if k < self.count / PARTIAL_SORT_THRESHOLD {
            // Use partial sort for small k
            results.select_nth_unstable_by(k - 1, |a, b| a.1.total_cmp(&b.1));
            results.truncate(k);
            results.sort_by(|a, b| a.1.total_cmp(&b.1));
        } else {
            // Full sort for large k
            results.sort_by(|a, b| a.1.total_cmp(&b.1));
            results.truncate(k);
        }

        Ok(results
            .into_iter()
            .map(|(id, distance)| BinaryFlatSearchResult { id, distance })
            .collect())
    }

    /// Get a vector by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Vector ID
    ///
    /// # Returns
    ///
    /// The vector bytes, or None if ID is out of bounds.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn get(&self, id: VectorId) -> Option<&[u8]> {
        // IDs are 1-based, so subtract 1 to get 0-based index
        // SAFETY: VectorId.0 is u64 but in practice never exceeds usize::MAX
        // on any supported platform (WASM32 or x86_64).
        let idx = (id.0 as usize).checked_sub(1)?;
        if idx >= self.count {
            return None;
        }
        let start = idx * self.bytes_per_vector;
        let end = start + self.bytes_per_vector;
        Some(&self.vectors[start..end])
    }

    /// Get the number of vectors in the index.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the index is empty.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get the dimensions (bits) per vector.
    #[must_use]
    #[inline]
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Get the bytes per vector.
    #[must_use]
    #[inline]
    pub fn bytes_per_vector(&self) -> usize {
        self.bytes_per_vector
    }

    /// Get approximate memory usage in bytes.
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.vectors.capacity()
    }

    /// Get the length of the internal vectors buffer.
    #[inline]
    #[must_use]
    pub fn vectors_len(&self) -> usize {
        self.vectors.len()
    }

    /// Estimate the serialized size in bytes.
    ///
    /// Format: header (dimensions u32 + count u32) + vector data.
    #[must_use]
    pub fn serialized_size(&self) -> usize {
        SERIALIZATION_HEADER_SIZE + self.vectors.len()
    }

    /// Clear all vectors from the index.
    pub fn clear(&mut self) {
        self.vectors.clear();
        self.count = 0;
    }

    /// Shrink the internal storage to fit the current number of vectors.
    pub fn shrink_to_fit(&mut self) {
        self.vectors.shrink_to_fit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let index = BinaryFlatIndex::new(1024).unwrap();
        assert_eq!(index.dimensions(), 1024);
        assert_eq!(index.bytes_per_vector(), 128);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_insert_and_get() {
        let mut index = BinaryFlatIndex::new(64).unwrap(); // 8 bytes per vector
        let v1 = vec![0xFF; 8];
        let v2 = vec![0x00; 8];

        let id1 = index.insert(&v1).unwrap();
        let id2 = index.insert(&v2).unwrap();

        assert_eq!(id1, VectorId(1)); // IDs are 1-based
        assert_eq!(id2, VectorId(2));
        assert_eq!(index.len(), 2);
        assert_eq!(index.get(id1), Some(v1.as_slice()));
        assert_eq!(index.get(id2), Some(v2.as_slice()));
        assert_eq!(index.get(VectorId(99)), None);
    }

    #[test]
    fn test_search_exact_match() {
        let mut index = BinaryFlatIndex::new(64).unwrap();

        // Insert some vectors
        let v1 = vec![0xFF; 8];
        let v2 = vec![0x00; 8];
        let v3 = vec![0xAA; 8];

        index.insert(&v1).unwrap();
        index.insert(&v2).unwrap();
        index.insert(&v3).unwrap();

        // Search for exact match
        let results = index.search(&v2, 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, VectorId(2)); // v2 is second insert, ID=2
        assert!((results[0].distance - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_search_ordering() {
        let mut index = BinaryFlatIndex::new(64).unwrap();

        // Insert vectors with known distances from query
        let query = vec![0x00; 8]; // All zeros
        let v1 = vec![0xFF; 8]; // 64 bits different
        let v2 = vec![0x0F; 8]; // 32 bits different
        let v3 = vec![0x01; 8]; // 8 bits different

        index.insert(&v1).unwrap(); // id=1, dist=64
        index.insert(&v2).unwrap(); // id=2, dist=32
        index.insert(&v3).unwrap(); // id=3, dist=8

        let results = index.search(&query, 3).unwrap();

        // Should be ordered by distance (IDs are 1-based)
        assert_eq!(results[0].id, VectorId(3)); // Closest
        assert!((results[0].distance - 8.0).abs() < f32::EPSILON);
        assert_eq!(results[1].id, VectorId(2));
        assert!((results[1].distance - 32.0).abs() < f32::EPSILON);
        assert_eq!(results[2].id, VectorId(1)); // Farthest
        assert!((results[2].distance - 64.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_search_k_limit() {
        let mut index = BinaryFlatIndex::new(64).unwrap();

        for i in 0..100 {
            let v: Vec<u8> = (0..8)
                .map(|j| u8::try_from((i + j) % 256).unwrap())
                .collect();
            index.insert(&v).unwrap();
        }

        let query = vec![0x00; 8];
        let results = index.search(&query, 5).unwrap();

        assert_eq!(results.len(), 5);
        // Results should be sorted by distance
        for i in 1..results.len() {
            assert!(results[i - 1].distance <= results[i].distance);
        }
    }

    #[test]
    fn test_empty_search() {
        let index = BinaryFlatIndex::new(64).unwrap();
        let query = vec![0x00; 8];
        let results = index.search(&query, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_k_zero() {
        let mut index = BinaryFlatIndex::new(64).unwrap();
        // Insert enough vectors to trigger the partial sort path (count >= 10)
        for _ in 0..20 {
            index.insert(&[0xFF; 8]).unwrap();
        }
        // k=0 should return empty, not error
        let results = index.search(&[0x00; 8], 0).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut index = BinaryFlatIndex::new(64).unwrap();
        index.insert(&[0xFF; 8]).unwrap();
        index.insert(&[0x00; 8]).unwrap();

        assert_eq!(index.len(), 2);

        index.clear();

        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_memory_usage() {
        let mut index = BinaryFlatIndex::with_capacity(1024, 1000).unwrap();
        assert!(index.memory_usage() > 0);

        for _ in 0..100 {
            index.insert(&[0xAA; 128]).unwrap();
        }

        let usage = index.memory_usage();
        // Should be at least 100 * 128 = 12,800 bytes
        assert!(usage >= 12_800);
    }

    #[test]
    fn test_invalid_dimensions() {
        let result = BinaryFlatIndex::new(100); // Not divisible by 8
        assert!(matches!(
            result,
            Err(BinaryFlatIndexError::InvalidDimensions(100))
        ));
    }

    #[test]
    fn test_invalid_vector_length() {
        let mut index = BinaryFlatIndex::new(64).unwrap();
        let result = index.insert(&[0xFF; 16]); // Wrong size
        assert!(matches!(
            result,
            Err(BinaryFlatIndexError::DimensionMismatch {
                expected: 8,
                actual: 16
            })
        ));
    }

    #[test]
    fn test_invalid_query_length() {
        let mut index = BinaryFlatIndex::new(64).unwrap();
        index.insert(&[0xFF; 8]).unwrap();
        let result = index.search(&[0x00; 16], 1); // Wrong query size
        assert!(matches!(
            result,
            Err(BinaryFlatIndexError::DimensionMismatch {
                expected: 8,
                actual: 16
            })
        ));
    }

    // ============= Phase 2 Coverage: vectors_len, serialized_size, shrink_to_fit =============

    #[test]
    fn test_vectors_len() {
        let mut index = BinaryFlatIndex::new(64).unwrap(); // 8 bytes per vector
        assert_eq!(index.vectors_len(), 0);

        index.insert(&[0xAA; 8]).unwrap();
        assert_eq!(index.vectors_len(), 8);
        assert_eq!(index.vectors_len(), index.len() * index.bytes_per_vector());

        index.insert(&[0xBB; 8]).unwrap();
        assert_eq!(index.vectors_len(), 16);
        assert_eq!(index.vectors_len(), index.len() * index.bytes_per_vector());

        index.insert(&[0xCC; 8]).unwrap();
        assert_eq!(index.vectors_len(), 24);
        assert_eq!(index.vectors_len(), index.len() * index.bytes_per_vector());

        // After clear, vectors_len must be 0
        index.clear();
        assert_eq!(index.vectors_len(), 0);
        assert_eq!(index.len(), 0);
        assert_eq!(index.vectors_len(), index.len() * index.bytes_per_vector());
    }

    #[test]
    fn test_serialized_size() {
        let mut index = BinaryFlatIndex::new(64).unwrap(); // 8 bytes per vector
                                                           // Empty index: header only (8 bytes for dimensions u32 + count u32)
        assert_eq!(index.serialized_size(), 8);

        index.insert(&[0xAA; 8]).unwrap();
        // Header (8) + 1 vector (8 bytes) = 16
        assert_eq!(index.serialized_size(), 8 + 8);

        index.insert(&[0xBB; 8]).unwrap();
        assert_eq!(index.serialized_size(), 8 + 16);

        // Verify the formula matches: SERIALIZATION_HEADER_SIZE + vectors.len()
        assert_eq!(
            index.serialized_size(),
            SERIALIZATION_HEADER_SIZE + index.vectors_len()
        );

        // After clear
        index.clear();
        assert_eq!(index.serialized_size(), SERIALIZATION_HEADER_SIZE);
    }

    #[test]
    fn test_shrink_to_fit() {
        // Create with large capacity, insert few vectors, then shrink
        let mut index = BinaryFlatIndex::with_capacity(1024, 10_000).unwrap();
        assert!(index.memory_usage() > 100_000); // Pre-allocated ~1.28MB

        // Insert a small number of vectors
        for _ in 0..10 {
            index.insert(&[0xAA; 128]).unwrap();
        }

        let before_shrink = index.memory_usage();
        index.shrink_to_fit();
        let after_shrink = index.memory_usage();

        // After shrink, memory usage should be <= before (likely much less)
        assert!(after_shrink <= before_shrink);

        // Data must still be intact
        assert_eq!(index.len(), 10);
        assert_eq!(index.vectors_len(), 10 * 128);
        for i in 1..=10_u64 {
            assert_eq!(index.get(VectorId(i)), Some([0xAA; 128].as_slice()));
        }

        // Shrink on empty index should not panic
        let mut empty = BinaryFlatIndex::new(64).unwrap();
        empty.shrink_to_fit();
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut index = BinaryFlatIndex::new(64).unwrap();
        index.insert(&[0xFF; 8]).unwrap();
        index.insert(&[0x00; 8]).unwrap();
        index.insert(&[0xAA; 8]).unwrap();

        // Serialize with serde_json (available as dev-dependency)
        let json = serde_json::to_string(&index).expect("serialize failed");
        let restored: BinaryFlatIndex = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(restored.dimensions(), index.dimensions());
        assert_eq!(restored.bytes_per_vector(), index.bytes_per_vector());
        assert_eq!(restored.len(), index.len());
        assert_eq!(restored.vectors_len(), index.vectors_len());

        // Verify all vectors match
        for i in 1..=3u64 {
            assert_eq!(
                restored.get(VectorId(i)),
                index.get(VectorId(i)),
                "vector {} mismatch after roundtrip",
                i
            );
        }
    }
}

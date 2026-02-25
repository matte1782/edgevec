//! Sparse vector search functionality.
//!
//! This module provides search capabilities over sparse vector storage,
//! using dot product similarity for ranking.
//!
//! # Algorithm
//!
//! Uses brute-force search: computes dot product between query and all
//! non-deleted vectors, returns top-k by score. For small collections
//! (<100k vectors), this is efficient due to sparse vector sparsity.
//!
//! # Performance Targets
//!
//! [HOSTILE_REVIEW: m1 Resolution] - Aligned with Day 6 benchmark targets
//! - 10k vectors, avg 50 nnz: <20ms (acceptable), <10ms (target)
//! - 100k vectors, avg 50 nnz: <100ms
//!
//! # Future Optimizations
//!
//! For larger collections, consider:
//! - Inverted index for posting list intersection
//! - SIMD acceleration for dot product
//! - Batch query processing

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::sparse::{sparse_dot_product, SparseId, SparseStorage, SparseVector};

// =============================================================================
// SEARCH RESULT TYPE
// =============================================================================

/// A search result from sparse search.
#[derive(Clone, Debug)]
pub struct SparseSearchResult {
    /// ID of the matched sparse vector
    pub id: SparseId,
    /// Similarity score (dot product)
    pub score: f32,
}

impl SparseSearchResult {
    /// Create a new search result.
    #[inline]
    #[must_use]
    pub fn new(id: SparseId, score: f32) -> Self {
        Self { id, score }
    }
}

// =============================================================================
// MIN-HEAP ENTRY (for top-k selection)
// =============================================================================

/// Wrapper for min-heap ordering (we want max-heap behavior for top-k).
///
/// By reversing the comparison, we can use Rust's max-heap (`BinaryHeap`)
/// as a min-heap, keeping the smallest element at the top for efficient pruning.
#[derive(Clone, Debug)]
struct MinHeapEntry {
    id: SparseId,
    score: f32,
}

impl PartialEq for MinHeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.id == other.id
    }
}

impl Eq for MinHeapEntry {}

impl PartialOrd for MinHeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MinHeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (smallest score at top)
        // This allows efficient pruning: if score < heap.peek(), skip
        match other.score.partial_cmp(&self.score) {
            Some(ord) => ord,
            None => Ordering::Equal, // Handle NaN by treating as equal
        }
    }
}

// =============================================================================
// SPARSE SEARCHER
// =============================================================================

/// Sparse vector search engine.
///
/// Performs brute-force search over a `SparseStorage` using sparse dot product.
/// For small collections (<100k vectors), brute force is efficient due to
/// sparse vector locality.
///
/// # Performance
///
/// - Search: O(n * avg_nnz) where n is live vector count
/// - Target: <20ms for 10k vectors with avg 50 nnz
///
/// # Example
///
/// ```rust
/// use edgevec::sparse::{SparseStorage, SparseVector, SparseSearcher};
///
/// let mut storage = SparseStorage::new();
///
/// // Insert some vectors
/// let v1 = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100)?;
/// let v2 = SparseVector::new(vec![0, 5, 20], vec![0.5, 1.5, 2.0], 100)?;
/// storage.insert(&v1)?;
/// storage.insert(&v2)?;
///
/// // Search
/// let searcher = SparseSearcher::new(&storage);
/// let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100)?;
/// let results = searcher.search(&query, 10);
///
/// for result in &results {
///     println!("ID: {:?}, Score: {}", result.id, result.score);
/// }
/// # Ok::<(), edgevec::sparse::SparseError>(())
/// ```
pub struct SparseSearcher<'a> {
    storage: &'a SparseStorage,
}

impl<'a> SparseSearcher<'a> {
    /// Create a new sparse searcher over the given storage.
    ///
    /// # Arguments
    ///
    /// * `storage` - Reference to the sparse storage to search
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseSearcher};
    ///
    /// let storage = SparseStorage::new();
    /// let searcher = SparseSearcher::new(&storage);
    /// ```
    #[inline]
    #[must_use]
    pub fn new(storage: &'a SparseStorage) -> Self {
        Self { storage }
    }

    /// Search for top-k most similar sparse vectors.
    ///
    /// Uses sparse dot product similarity (higher = more similar).
    /// Vectors with zero dot product (no overlapping indices) are excluded.
    ///
    /// # Algorithm
    ///
    /// 1. Iterate over all non-deleted vectors in storage
    /// 2. Compute dot product with query
    /// 3. Maintain top-k heap
    /// 4. Return sorted results
    ///
    /// # Arguments
    ///
    /// * `query` - Sparse query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// Vec of `SparseSearchResult` (id, score) pairs, sorted by descending score.
    /// May return fewer than k results if storage has fewer live vectors.
    ///
    /// # Complexity
    ///
    /// - Time: O(n * avg_nnz + k * log(k)) where n is live vector count
    /// - Space: O(k) for the result heap
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::{SparseStorage, SparseVector, SparseSearcher};
    ///
    /// let mut storage = SparseStorage::new();
    /// let v1 = SparseVector::new(vec![0, 5], vec![1.0, 2.0], 100)?;
    /// storage.insert(&v1)?;
    ///
    /// let searcher = SparseSearcher::new(&storage);
    /// let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100)?;
    /// let results = searcher.search(&query, 10);
    ///
    /// assert_eq!(results.len(), 1);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn search(&self, query: &SparseVector, k: usize) -> Vec<SparseSearchResult> {
        if k == 0 {
            return Vec::new();
        }

        // Use min-heap to maintain top-k
        // The smallest score is at the top, so we can efficiently prune
        let mut heap: BinaryHeap<MinHeapEntry> = BinaryHeap::with_capacity(k + 1);

        // Iterate over all non-deleted vectors
        for (id, vector) in self.storage {
            let score = sparse_dot_product(query, &vector);

            // Skip vectors with no overlap (score = 0)
            if score <= 0.0 {
                continue;
            }

            // Check if this score is good enough to enter the heap
            if heap.len() < k {
                heap.push(MinHeapEntry { id, score });
            } else if let Some(min_entry) = heap.peek() {
                if score > min_entry.score {
                    heap.pop();
                    heap.push(MinHeapEntry { id, score });
                }
            }
        }

        // Extract results and sort by descending score
        let mut results: Vec<SparseSearchResult> = heap
            .into_iter()
            .map(|entry| SparseSearchResult::new(entry.id, entry.score))
            .collect();

        // Sort descending by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

        results
    }

    /// Search and return raw (SparseId, f32) tuples.
    ///
    /// Convenience method returning `Vec<(SparseId, f32)>` for easier
    /// integration with other systems.
    ///
    /// # Arguments
    ///
    /// * `query` - Sparse query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// Vec of (SparseId, score) tuples, sorted by descending score.
    #[must_use]
    pub fn search_raw(&self, query: &SparseVector, k: usize) -> Vec<(SparseId, f32)> {
        self.search(query, k)
            .into_iter()
            .map(|r| (r.id, r.score))
            .collect()
    }

    /// Search and return raw (u64, f32) tuples for fusion compatibility.
    ///
    /// Converts `SparseId` to `u64` for use with fusion algorithms that
    /// need a common ID type between dense and sparse results.
    ///
    /// # Arguments
    ///
    /// * `query` - Sparse query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// Vec of (u64, score) tuples, sorted by descending score.
    #[must_use]
    pub fn search_u64(&self, query: &SparseVector, k: usize) -> Vec<(u64, f32)> {
        self.search(query, k)
            .into_iter()
            .map(|r| (r.id.as_u64(), r.score))
            .collect()
    }

    /// Get reference to underlying storage.
    #[inline]
    #[must_use]
    pub fn storage(&self) -> &SparseStorage {
        self.storage
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ============= Helper Functions =============

    fn create_test_storage() -> SparseStorage {
        let mut storage = SparseStorage::new();

        // v0: [0, 5, 10] = [1.0, 2.0, 3.0] - high overlap with typical query
        let v0 = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        storage.insert(&v0).unwrap();

        // v1: [5, 10, 20] = [0.5, 1.5, 2.0] - medium overlap
        let v1 = SparseVector::new(vec![5, 10, 20], vec![0.5, 1.5, 2.0], 100).unwrap();
        storage.insert(&v1).unwrap();

        // v2: [30, 40, 50] = [1.0, 1.0, 1.0] - no overlap with [0, 5, 10]
        let v2 = SparseVector::new(vec![30, 40, 50], vec![1.0, 1.0, 1.0], 100).unwrap();
        storage.insert(&v2).unwrap();

        // v3: [0] = [5.0] - single element, high weight on index 0
        let v3 = SparseVector::new(vec![0], vec![5.0], 100).unwrap();
        storage.insert(&v3).unwrap();

        storage
    }

    // ============= Basic Search Tests =============

    #[test]
    fn test_search_basic() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        // Query: [0, 5] = [1.0, 1.0]
        // Expected scores:
        // v0: 1.0*1.0 + 2.0*1.0 = 3.0
        // v1: 0.5*1.0 = 0.5 (only index 5 matches)
        // v2: 0.0 (no overlap)
        // v3: 5.0*1.0 = 5.0 (only index 0 matches)
        let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100).unwrap();
        let results = searcher.search(&query, 10);

        assert_eq!(results.len(), 3); // v2 excluded (no overlap)

        // Check ordering: v3 (5.0) > v0 (3.0) > v1 (0.5)
        assert_eq!(results[0].id.as_u64(), 3);
        assert!((results[0].score - 5.0).abs() < 1e-6);

        assert_eq!(results[1].id.as_u64(), 0);
        assert!((results[1].score - 3.0).abs() < 1e-6);

        assert_eq!(results[2].id.as_u64(), 1);
        assert!((results[2].score - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_search_empty_storage() {
        let storage = SparseStorage::new();
        let searcher = SparseSearcher::new(&storage);

        let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100).unwrap();
        let results = searcher.search(&query, 10);

        assert!(results.is_empty());
    }

    #[test]
    fn test_search_k_zero() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100).unwrap();
        let results = searcher.search(&query, 0);

        assert!(results.is_empty());
    }

    #[test]
    fn test_search_k_larger_than_count() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100).unwrap();
        let results = searcher.search(&query, 1000);

        // Should return only matching vectors (3), not 1000
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_k_equals_one() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100).unwrap();
        let results = searcher.search(&query, 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.as_u64(), 3); // Highest score
    }

    // ============= Deleted Vector Tests =============

    #[test]
    fn test_search_skips_deleted() {
        let mut storage = create_test_storage();

        // Query matching v3
        let query = SparseVector::new(vec![0], vec![1.0], 100).unwrap();

        // Before deletion
        let searcher = SparseSearcher::new(&storage);
        let results_before = searcher.search(&query, 10);
        assert!(results_before.iter().any(|r| r.id.as_u64() == 3));

        // Delete v3 (id=3)
        storage.delete(SparseId::new(3)).unwrap();

        // Create new searcher with updated storage view
        let searcher = SparseSearcher::new(&storage);
        let results_after = searcher.search(&query, 10);

        // v3 should be excluded
        assert!(!results_after.iter().any(|r| r.id.as_u64() == 3));
    }

    // ============= Ordering Tests =============

    #[test]
    fn test_search_ordering_descending() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        let query = SparseVector::new(vec![0, 5, 10], vec![1.0, 1.0, 1.0], 100).unwrap();
        let results = searcher.search(&query, 10);

        // Check all scores are in descending order
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results not sorted: {} < {}",
                results[i - 1].score,
                results[i].score
            );
        }
    }

    #[test]
    fn test_search_raw_format() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100).unwrap();
        let results = searcher.search_raw(&query, 10);

        // Check format is (SparseId, f32)
        assert!(!results.is_empty());
        assert_eq!(results[0].0.as_u64(), 3);
    }

    #[test]
    fn test_search_u64_format() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100).unwrap();
        let results = searcher.search_u64(&query, 10);

        // Check format is (u64, f32)
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 3u64);
    }

    // ============= No Overlap Tests =============

    #[test]
    fn test_search_no_overlap_returns_empty() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        // Query with indices that don't exist in any vector
        let query = SparseVector::new(vec![99], vec![1.0], 100).unwrap();
        let results = searcher.search(&query, 10);

        assert!(results.is_empty());
    }

    // ============= Score Correctness Tests =============

    #[test]
    fn test_search_score_correctness() {
        let mut storage = SparseStorage::new();

        // Insert vectors with known dot products
        let v0 = SparseVector::new(vec![0, 1, 2], vec![1.0, 2.0, 3.0], 10).unwrap();
        let v1 = SparseVector::new(vec![1, 2, 3], vec![4.0, 5.0, 6.0], 10).unwrap();
        storage.insert(&v0).unwrap();
        storage.insert(&v1).unwrap();

        let searcher = SparseSearcher::new(&storage);

        // Query: [1, 2] = [1.0, 1.0]
        // v0 dot: 2.0*1.0 + 3.0*1.0 = 5.0
        // v1 dot: 4.0*1.0 + 5.0*1.0 = 9.0
        let query = SparseVector::new(vec![1, 2], vec![1.0, 1.0], 10).unwrap();
        let results = searcher.search(&query, 10);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id.as_u64(), 1); // v1 has higher score
        assert!((results[0].score - 9.0).abs() < 1e-6);
        assert_eq!(results[1].id.as_u64(), 0);
        assert!((results[1].score - 5.0).abs() < 1e-6);
    }

    // ============= Large Scale Tests =============

    #[test]
    fn test_search_many_vectors() {
        let mut storage = SparseStorage::new();

        // Insert 1000 vectors, all with overlapping index 0
        // Vector i has indices [0, i+1] so all overlap with query [0]
        for i in 0..1000 {
            let v = SparseVector::new(vec![0, u32::try_from(i + 1).unwrap()], vec![1.0, 1.0], 2000)
                .unwrap();
            storage.insert(&v).unwrap();
        }

        let searcher = SparseSearcher::new(&storage);

        // Query [0, 1] = [1.0, 2.0]
        // v0 has [0, 1]: score = 1.0*1.0 + 1.0*2.0 = 3.0 (best match)
        // All others: score = 1.0*1.0 = 1.0 (only index 0 matches)
        let query = SparseVector::new(vec![0, 1], vec![1.0, 2.0], 2000).unwrap();
        let results = searcher.search(&query, 10);

        assert_eq!(results.len(), 10);
        // First result should be v0 (score = 3.0, has both indices)
        assert_eq!(results[0].id.as_u64(), 0);
        assert!((results[0].score - 3.0).abs() < 1e-6);
    }

    // ============= Storage Reference Test =============

    #[test]
    fn test_storage_accessor() {
        let storage = create_test_storage();
        let searcher = SparseSearcher::new(&storage);

        // Verify storage reference works
        assert_eq!(searcher.storage().len(), 4);
    }
}

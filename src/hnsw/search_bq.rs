#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

//! Binary quantization search for HNSW (v0.7.0 - RFC-002 Phase 2).
//!
//! This module provides fast approximate search using Hamming distance
//! on binary quantized vectors. BQ search is 3-5x faster than F32 search
//! but has lower recall (typically 0.85-0.95 depending on data).
//!
//! # Algorithm
//!
//! 1. Quantize query vector to binary
//! 2. Traverse HNSW graph using Hamming distance instead of F32
//! 3. Return top-k candidates sorted by Hamming distance
//! 4. Convert Hamming distance to approximate similarity
//!
//! # Example
//!
//! ```
//! use edgevec::hnsw::{HnswConfig, HnswIndex};
//! use edgevec::storage::VectorStorage;
//!
//! let config = HnswConfig::new(128);
//! let mut storage = VectorStorage::new(&config, None);
//! let mut index = HnswIndex::with_bq(config, &storage).unwrap();
//!
//! // Insert vectors
//! let v = vec![1.0f32; 128];
//! index.insert_bq(&v, &mut storage).unwrap();
//!
//! // Search with BQ (fast approximate)
//! let query = vec![1.0f32; 128];
//! let results = index.search_bq(&query, 10, &storage).unwrap();
//! ```

use super::graph::{GraphError, HnswIndex, NodeId, VectorId};
use crate::quantization::variable::BinaryVector;
use crate::simd::popcount::simd_popcount_xor;
use crate::storage::binary::BinaryVectorStorage;
use crate::storage::VectorStorage;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};

/// A candidate for BQ search, ordered by Hamming distance.
#[derive(Clone, Copy, Debug)]
struct BqCandidate {
    /// Hamming distance to the query vector.
    distance: u32,
    /// The node ID.
    node_id: NodeId,
}

impl PartialEq for BqCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.node_id == other.node_id
    }
}

impl Eq for BqCandidate {}

impl PartialOrd for BqCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BqCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.distance.cmp(&other.distance)
    }
}

impl HnswIndex {
    /// Searches the index using binary quantization (Hamming distance).
    ///
    /// This is faster than F32 search but has lower recall.
    /// Use `search_bq_rescored()` for better recall with F32 rescoring.
    ///
    /// # Arguments
    ///
    /// * `query` - The query vector.
    /// * `k` - Number of results to return.
    /// * `_storage` - The F32 storage (needed for compatibility, not used).
    ///
    /// # Returns
    ///
    /// Top-k results sorted by approximate similarity (higher is better).
    ///
    /// # Errors
    ///
    /// - `GraphError::BqNotEnabled` if BQ storage is not initialized.
    /// - `GraphError::DimensionMismatch` if query dimension is wrong.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(128);
    /// let mut storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::with_bq(config, &storage).unwrap();
    ///
    /// let v = vec![1.0f32; 128];
    /// index.insert_bq(&v, &mut storage).unwrap();
    ///
    /// let query = vec![1.0f32; 128];
    /// let results = index.search_bq(&query, 10, &storage).unwrap();
    ///
    /// assert_eq!(results.len(), 1);
    /// assert!((results[0].1 - 1.0).abs() < 0.01); // Similarity ~1.0
    /// ```
    pub fn search_bq(
        &self,
        query: &[f32],
        k: usize,
        _storage: &VectorStorage,
    ) -> Result<Vec<(VectorId, f32)>, GraphError> {
        // Validate BQ is enabled
        let bq_storage = self.bq_storage.as_ref().ok_or(GraphError::BqNotEnabled)?;

        // Validate dimension
        let expected_dim = self.config.dimensions as usize;
        if query.len() != expected_dim {
            return Err(GraphError::DimensionMismatch {
                expected: expected_dim,
                actual: query.len(),
            });
        }

        // Check for empty index
        if self.entry_point.is_none() {
            return Ok(Vec::new());
        }

        // Quantize query
        let query_bq =
            BinaryVector::quantize(query).map_err(|e| GraphError::Quantization(e.to_string()))?;

        // Use existing HNSW traversal with Hamming distance
        let candidates = self.search_bq_internal(&query_bq, k, bq_storage)?;

        // Convert to similarity scores
        let dimension = expected_dim as f32;
        let results: Vec<_> = candidates
            .into_iter()
            .map(|(id, hamming_dist)| {
                // Similarity = 1 - (hamming_dist / dimension)
                let similarity = 1.0 - (hamming_dist as f32 / dimension);
                (id, similarity)
            })
            .collect();

        Ok(results)
    }

    /// Searches using binary quantization with F32 rescoring.
    ///
    /// This provides the best of both worlds:
    /// - Fast BQ search for candidate generation
    /// - Accurate F32 rescoring for final ranking
    ///
    /// # Arguments
    ///
    /// * `query` - The query vector.
    /// * `k` - Number of results to return.
    /// * `rescore_factor` - Overfetch multiplier (recommended: 3).
    ///   Higher values improve recall but increase latency.
    /// * `storage` - F32 vector storage.
    ///
    /// # Returns
    ///
    /// Top-k results sorted by exact F32 distance (converted to similarity).
    ///
    /// # Errors
    ///
    /// - `GraphError::BqNotEnabled` if BQ storage is not initialized.
    /// - `GraphError::DimensionMismatch` if query dimension is wrong.
    ///
    /// # Performance
    ///
    /// - BQ phase: O(log n × d/8) — very fast
    /// - Rescore phase: O(k × rescore_factor × d) — proportional to overfetch
    ///
    /// Typical latency: 1.5-2× pure BQ, but with recall ~0.95+
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(128);
    /// let mut storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::with_bq(config, &storage).unwrap();
    ///
    /// let v = vec![1.0f32; 128];
    /// index.insert_bq(&v, &mut storage).unwrap();
    ///
    /// // Search for 10 results with 3× overfetch
    /// let results = index.search_bq_rescored(&v, 10, 3, &storage).unwrap();
    /// ```
    pub fn search_bq_rescored(
        &self,
        query: &[f32],
        k: usize,
        rescore_factor: usize,
        storage: &VectorStorage,
    ) -> Result<Vec<(VectorId, f32)>, GraphError> {
        use super::rescore::rescore_top_k;

        // Validate inputs
        let rescore_factor = rescore_factor.max(1); // At least 1×

        // Step 1: BQ search for more candidates
        let overfetched_k = k.saturating_mul(rescore_factor);
        let bq_candidates = self.search_bq(query, overfetched_k, storage)?;

        // Step 2: Rescore with F32 and return top-k
        let rescored = rescore_top_k(&bq_candidates, query, storage, k);

        // Convert distance to similarity for consistent API
        // (Lower distance = higher similarity)
        let results: Vec<_> = rescored
            .into_iter()
            .map(|(id, distance)| {
                // Convert distance to similarity
                // Using inverse: similarity = 1 / (1 + distance)
                let similarity = 1.0 / (1.0 + distance);
                (id, similarity)
            })
            .collect();

        Ok(results)
    }

    /// Convenience method with default rescore factor.
    ///
    /// Uses rescore_factor = 5, which provides good recall/speed balance.
    /// For maximum recall (>0.90), use rescore_factor = 10 or higher.
    ///
    /// # Errors
    ///
    /// - `GraphError::BqNotEnabled` if BQ storage is not initialized.
    /// - `GraphError::DimensionMismatch` if query dimension is wrong.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(128);
    /// let mut storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::with_bq(config, &storage).unwrap();
    ///
    /// let v = vec![1.0f32; 128];
    /// index.insert_bq(&v, &mut storage).unwrap();
    ///
    /// let results = index.search_bq_rescored_default(&v, 10, &storage).unwrap();
    /// ```
    pub fn search_bq_rescored_default(
        &self,
        query: &[f32],
        k: usize,
        storage: &VectorStorage,
    ) -> Result<Vec<(VectorId, f32)>, GraphError> {
        self.search_bq_rescored(query, k, 5, storage)
    }

    /// High-recall BQ search (rescore_factor = 15).
    ///
    /// Use this when recall is critical and latency is acceptable.
    /// Achieves >0.90 recall on most datasets.
    ///
    /// # Errors
    ///
    /// - `GraphError::BqNotEnabled` if BQ storage is not initialized.
    /// - `GraphError::DimensionMismatch` if query dimension is wrong.
    pub fn search_bq_high_recall(
        &self,
        query: &[f32],
        k: usize,
        storage: &VectorStorage,
    ) -> Result<Vec<(VectorId, f32)>, GraphError> {
        self.search_bq_rescored(query, k, 15, storage)
    }

    /// Internal BQ search using Hamming distance.
    fn search_bq_internal(
        &self,
        query_bq: &BinaryVector,
        k: usize,
        bq_storage: &BinaryVectorStorage,
    ) -> Result<Vec<(VectorId, u32)>, GraphError> {
        let entry = self.entry_point.ok_or(GraphError::BqNotEnabled)?;
        let mut current = entry;

        // Descend from max_layer to layer 1
        for layer in (1..=self.max_layer).rev() {
            current = self.greedy_bq(current, query_bq, layer, bq_storage)?;
        }

        // Search layer 0 for k nearest
        let candidates = self.search_layer_bq(current, query_bq, k, bq_storage)?;

        Ok(candidates)
    }

    /// Greedy descent using Hamming distance.
    fn greedy_bq(
        &self,
        start: NodeId,
        query_bq: &BinaryVector,
        layer: u8,
        bq_storage: &BinaryVectorStorage,
    ) -> Result<NodeId, GraphError> {
        let mut current = start;
        let mut current_dist = self.hamming_to_node(query_bq, current, bq_storage)?;

        loop {
            let mut changed = false;

            let node = self
                .get_node(current)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            if node.max_layer < layer {
                break;
            }

            let neighbors = self.get_neighbors_at_layer(node, layer)?;
            for neighbor in neighbors {
                let Some(neighbor_node) = self.get_node(neighbor) else {
                    continue;
                };

                // Skip if neighbor is deleted
                if neighbor_node.deleted != 0 {
                    continue;
                }

                // Skip if neighbor doesn't exist at this layer
                if neighbor_node.max_layer < layer {
                    continue;
                }

                let dist = self.hamming_to_node(query_bq, neighbor, bq_storage)?;
                if dist < current_dist {
                    current = neighbor;
                    current_dist = dist;
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        Ok(current)
    }

    /// Search layer 0 for k nearest using Hamming distance.
    fn search_layer_bq(
        &self,
        entry: NodeId,
        query_bq: &BinaryVector,
        k: usize,
        bq_storage: &BinaryVectorStorage,
    ) -> Result<Vec<(VectorId, u32)>, GraphError> {
        // Use ef_search (not ef_construction) for search beam width
        // Higher ef_search = better recall but slower search
        let ef = self.config.ef_search.max(k as u32) as usize;

        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut candidates: BinaryHeap<Reverse<BqCandidate>> = BinaryHeap::new();
        let mut results: BinaryHeap<BqCandidate> = BinaryHeap::new();

        // Initialize with entry point
        let entry_dist = self.hamming_to_node(query_bq, entry, bq_storage)?;
        let entry_node = self.get_node(entry).ok_or(GraphError::NodeIdOutOfBounds)?;

        visited.insert(entry);
        candidates.push(Reverse(BqCandidate {
            distance: entry_dist,
            node_id: entry,
        }));

        // Only add to results if not deleted
        if entry_node.deleted == 0 {
            results.push(BqCandidate {
                distance: entry_dist,
                node_id: entry,
            });
        }

        // Greedy expansion
        while let Some(Reverse(candidate)) = candidates.pop() {
            // Get worst result distance
            let worst_dist = if results.len() >= ef {
                results.peek().map_or(u32::MAX, |c| c.distance)
            } else {
                u32::MAX
            };

            // If candidate is farther than worst result, we're done
            if candidate.distance > worst_dist {
                break;
            }

            // Get neighbors at layer 0
            let Some(node) = self.get_node(candidate.node_id) else {
                continue;
            };
            let neighbors = self.get_neighbors_at_layer(node, 0)?;

            for neighbor in neighbors {
                if visited.contains(&neighbor) {
                    continue;
                }
                visited.insert(neighbor);

                let Some(neighbor_node) = self.get_node(neighbor) else {
                    continue;
                };

                let dist = self.hamming_to_node(query_bq, neighbor, bq_storage)?;

                // Add to candidates if better than worst result
                let should_add =
                    results.len() < ef || dist < results.peek().map_or(u32::MAX, |c| c.distance);

                if should_add {
                    candidates.push(Reverse(BqCandidate {
                        distance: dist,
                        node_id: neighbor,
                    }));

                    // Add to results if not deleted
                    if neighbor_node.deleted == 0 {
                        results.push(BqCandidate {
                            distance: dist,
                            node_id: neighbor,
                        });

                        // Trim results to ef
                        while results.len() > ef {
                            results.pop();
                        }
                    }
                }
            }
        }

        // Extract top-k results (filter out any invalid node references)
        let mut result_vec: Vec<_> = results
            .into_iter()
            .filter_map(|c| {
                let node = self.nodes.get(c.node_id.0 as usize)?;
                Some((node.vector_id, c.distance))
            })
            .collect();

        // Sort by distance (ascending) and take k
        result_vec.sort_by_key(|(_, d)| *d);
        result_vec.truncate(k);

        Ok(result_vec)
    }

    /// Hamming distance from query to a node.
    fn hamming_to_node(
        &self,
        query_bq: &BinaryVector,
        node: NodeId,
        bq_storage: &BinaryVectorStorage,
    ) -> Result<u32, GraphError> {
        let idx = node.0 as usize;
        if idx >= self.nodes.len() {
            return Err(GraphError::NodeIdOutOfBounds);
        }

        // BQ storage is indexed by insertion order (0-based)
        // which matches node index
        let node_data = bq_storage
            .get_raw(u64::from(node.0))
            .ok_or(GraphError::InvalidVectorId)?;

        // Direct XOR popcount without constructing BinaryVector
        let dist = simd_popcount_xor(query_bq.data(), node_data);
        Ok(dist)
    }

    /// Gets neighbors at a specific layer.
    fn get_neighbors_at_layer(
        &self,
        node: &super::graph::HnswNode,
        layer: u8,
    ) -> Result<Vec<NodeId>, GraphError> {
        if layer == 0 {
            // For layer 0, return all neighbors
            self.get_neighbors(node)
        } else {
            // For higher layers, we need to decode the layered neighbor structure
            // For now, just use the general neighbors (this is a simplification)
            self.get_neighbors(node)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hnsw::config::HnswConfig;

    #[test]
    fn test_search_bq_empty_index() {
        let config = HnswConfig::new(128);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::with_bq(config, &storage).unwrap();

        let query = vec![1.0f32; 128];
        let results = index.search_bq(&query, 10, &storage).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_search_bq_not_enabled_error() {
        let config = HnswConfig::new(128);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::new(config, &storage).unwrap();

        let query = vec![1.0f32; 128];
        let result = index.search_bq(&query, 10, &storage);

        assert!(matches!(result, Err(GraphError::BqNotEnabled)));
    }

    #[test]
    fn test_search_bq_dimension_mismatch() {
        let config = HnswConfig::new(128);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::with_bq(config, &storage).unwrap();

        let query = vec![1.0f32; 64]; // Wrong dimension
        let result = index.search_bq(&query, 10, &storage);

        assert!(matches!(
            result,
            Err(GraphError::DimensionMismatch {
                expected: 128,
                actual: 64
            })
        ));
    }

    #[test]
    fn test_search_bq_single_vector() {
        let config = HnswConfig::new(128);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).unwrap();

        let v = vec![1.0f32; 128];
        index.insert_bq(&v, &mut storage).unwrap();

        let query = vec![1.0f32; 128];
        let results = index.search_bq(&query, 10, &storage).unwrap();

        assert_eq!(results.len(), 1);
        // Similarity should be very high (close to 1.0)
        assert!(results[0].1 > 0.99);
    }

    #[test]
    fn test_search_bq_finds_most_similar() {
        let config = HnswConfig::new(128);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).unwrap();

        // Insert vectors
        let v1 = vec![1.0f32; 128]; // All positive
        let v2 = vec![-1.0f32; 128]; // All negative
        let v3: Vec<f32> = (0..128).map(|i| if i < 64 { 1.0 } else { -1.0 }).collect(); // Half

        index.insert_bq(&v1, &mut storage).unwrap();
        index.insert_bq(&v2, &mut storage).unwrap();
        index.insert_bq(&v3, &mut storage).unwrap();

        // Query similar to v1
        let query = vec![1.0f32; 128];
        let results = index.search_bq(&query, 3, &storage).unwrap();

        // v1 should be most similar (Hamming = 0)
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0 .0, 1); // v1 has VectorId(1)
        assert!((results[0].1 - 1.0).abs() < 0.01); // Similarity ~1.0
    }

    #[test]
    fn test_search_bq_multiple_vectors() {
        let config = HnswConfig::new(128);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::with_bq(config, &storage).unwrap();

        // Insert 10 vectors
        for i in 0..10 {
            let v: Vec<f32> = (0..128)
                .map(|j| ((i * 128 + j) % 256) as f32 / 128.0 - 1.0)
                .collect();
            index.insert_bq(&v, &mut storage).unwrap();
        }

        // Search for top 5
        let query: Vec<f32> = (0..128).map(|j| (j % 256) as f32 / 128.0 - 1.0).collect();
        let results = index.search_bq(&query, 5, &storage).unwrap();

        assert_eq!(results.len(), 5);
        // Results should be sorted by similarity (descending)
        for i in 1..results.len() {
            assert!(results[i - 1].1 >= results[i].1);
        }
    }
}

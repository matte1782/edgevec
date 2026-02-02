use super::config::HnswConfig;
use super::graph::{GraphError, HnswIndex, NodeId, VectorId, VectorProvider};
use crate::metric::simd::l2_squared_u8;
use crate::metric::{DotProduct, Hamming, L2Squared, Metric};
use crate::storage::VectorStorage;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashSet};
use std::marker::PhantomData;

const MAX_TRAVERSAL_MULT: usize = 10;

/// Result of a search query.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The ID of the matching vector.
    pub vector_id: VectorId,
    /// The distance from the query vector.
    pub distance: f32,
}

/// A candidate node for search, containing its distance to the query and its ID.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    /// Distance to the query vector.
    pub distance: f32,
    /// The node ID.
    pub node_id: NodeId,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.node_id == other.node_id
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.distance.total_cmp(&other.distance)
    }
}

/// Context for search operations to reuse allocations.
pub struct SearchContext {
    /// Set of visited nodes to avoid cycles and re-processing.
    pub visited: HashSet<NodeId>,
    /// Min-heap of candidates to explore (nearest first).
    pub candidates: BinaryHeap<Reverse<Candidate>>,
    /// Max-heap of current top-k results (furthest first).
    pub results: BinaryHeap<Candidate>,
    /// Scratch buffer for neighbor selection (avoiding re-allocations).
    pub scratch: Vec<Candidate>,
    /// Scratch buffer for neighbor IDs (avoiding re-allocations).
    pub neighbor_scratch: Vec<NodeId>,
    /// Scratch buffer for raw neighbor IDs (u32) during decoding.
    pub neighbor_id_scratch: Vec<u32>,
    /// Scratch buffer for encoding.
    pub encoding_scratch: Vec<u8>,
    /// Quantized query buffer.
    pub quantized_query: Vec<u8>,
}

impl SearchContext {
    /// Creates a new search context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            visited: HashSet::new(),
            candidates: BinaryHeap::new(),
            results: BinaryHeap::new(),
            scratch: Vec::new(),
            neighbor_scratch: Vec::new(),
            neighbor_id_scratch: Vec::new(),
            encoding_scratch: Vec::new(),
            quantized_query: Vec::new(),
        }
    }

    /// Clears the context for reuse.
    pub fn clear(&mut self) {
        self.visited.clear();
        self.candidates.clear();
        self.results.clear();
        self.scratch.clear();
        self.neighbor_scratch.clear();
        self.neighbor_id_scratch.clear();
        self.encoding_scratch.clear();
        // Do NOT clear quantized_query here, as it is reused across layers for the same query.
    }
}

impl Default for SearchContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Structure to perform search on HNSW graph.
pub struct Searcher<'a, M, P: VectorProvider + ?Sized> {
    graph: &'a HnswIndex,
    provider: &'a P,
    _phantom: PhantomData<M>,
}

impl<'a, M, P: VectorProvider + ?Sized> Searcher<'a, M, P>
where
    M: Metric<f32>,
{
    /// Creates a new searcher.
    pub fn new(graph: &'a HnswIndex, provider: &'a P) -> Self {
        Self {
            graph,
            provider,
            _phantom: PhantomData,
        }
    }

    /// Performs a greedy search on a specific layer.
    ///
    /// # Errors
    /// Returns `GraphError` if node IDs are invalid or neighbor data is corrupted.
    #[inline]
    #[allow(clippy::cast_precision_loss)]
    pub fn search_layer(
        &self,
        ctx: &mut SearchContext,
        entry_points: impl IntoIterator<Item = NodeId>,
        query: &[f32],
        ef: usize,
        level: u8,
    ) -> Result<(), GraphError> {
        ctx.clear();

        // 0. Prepare quantization if applicable
        // Only use quantized path for L2Squared metric currently
        let use_quantized = if self.graph.config.metric == HnswConfig::METRIC_L2_SQUARED {
            if ctx.quantized_query.is_empty() {
                self.provider
                    .quantize_query(query, &mut ctx.quantized_query);
            }
            !ctx.quantized_query.is_empty()
        } else {
            false
        };

        // 1. Initialize
        for ep in entry_points {
            let node = self
                .graph
                .get_node(ep)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            if node.max_layer < level {
                continue;
            }

            let dist = if use_quantized {
                if let Some(q_vec) = self.provider.get_quantized_vector(node.vector_id) {
                    l2_squared_u8(&ctx.quantized_query, q_vec) as f32
                } else {
                    let vector = self.provider.get_vector(node.vector_id);
                    M::distance(query, &vector)
                }
            } else {
                let vector = self.provider.get_vector(node.vector_id);
                M::distance(query, &vector)
            };

            let candidate = Candidate {
                distance: dist,
                node_id: ep,
            };

            ctx.candidates.push(Reverse(candidate));
            // W16.3: Check node's deleted field directly instead of provider
            // This ensures deleted vectors are excluded from results but still used for routing
            if node.deleted == 0 {
                ctx.results.push(candidate);
            }
            ctx.visited.insert(ep);
        }

        // Prune initial
        while ctx.results.len() > ef {
            ctx.results.pop();
        }

        let traversal_limit = ef.saturating_mul(MAX_TRAVERSAL_MULT);
        let mut traversed_count = 0;

        // 2. Greedy Search
        while let Some(Reverse(candidate)) = ctx.candidates.pop() {
            traversed_count += 1;
            if traversed_count > traversal_limit {
                log::warn!(
                    "HNSW search traversal limit exceeded (ef={ef}, limit={traversal_limit}). Stopping early."
                );
                break;
            }

            if let Some(furthest) = ctx.results.peek() {
                if candidate.distance > furthest.distance && ctx.results.len() >= ef {
                    break;
                }
            }

            let node = self
                .graph
                .get_node(candidate.node_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;

            // Access neighbors via pub(crate) fields
            let start = node.neighbor_offset as usize;
            let end = start + node.neighbor_len as usize;

            if end > self.graph.neighbors.buffer.len() {
                return Err(GraphError::NeighborError);
            }

            let slice = &self.graph.neighbors.buffer[start..end];
            let neighbor_iter = crate::hnsw::neighbor::NeighborPool::iter_layer(slice, level);

            for neighbor_id_u32 in neighbor_iter {
                let neighbor_id = NodeId(neighbor_id_u32);
                if !ctx.visited.contains(&neighbor_id) {
                    ctx.visited.insert(neighbor_id);

                    let neighbor_node = self
                        .graph
                        .get_node(neighbor_id)
                        .ok_or(GraphError::NodeIdOutOfBounds)?;

                    let dist = if use_quantized {
                        if let Some(q_vec) =
                            self.provider.get_quantized_vector(neighbor_node.vector_id)
                        {
                            l2_squared_u8(&ctx.quantized_query, q_vec) as f32
                        } else {
                            let vector_data = self.provider.get_vector(neighbor_node.vector_id);
                            M::distance(query, &vector_data)
                        }
                    } else {
                        let vector_data = self.provider.get_vector(neighbor_node.vector_id);
                        M::distance(query, &vector_data)
                    };

                    let mut should_add = false;
                    if ctx.results.len() < ef {
                        should_add = true;
                    } else if let Some(furthest) = ctx.results.peek() {
                        if dist < furthest.distance {
                            should_add = true;
                        }
                    }

                    if should_add {
                        let new_candidate = Candidate {
                            distance: dist,
                            node_id: neighbor_id,
                        };
                        // W16.3: Always add to candidates for routing (deleted nodes are "ghosts")
                        ctx.candidates.push(Reverse(new_candidate));

                        // W16.3: Only add to results if not deleted (tombstone filtering)
                        if neighbor_node.deleted == 0 {
                            ctx.results.push(new_candidate);

                            if ctx.results.len() > ef {
                                ctx.results.pop();
                            }
                        }
                    }
                }
            }
        }

        // Return sorted results while preserving ctx capacity
        // Reuse scratch buffer
        while let Some(c) = ctx.results.pop() {
            ctx.scratch.push(c);
        }
        // MaxHeap returns largest first, so reverse to get ascending order
        ctx.scratch.reverse();

        Ok(())
    }
}

impl HnswIndex {
    /// Searches the index for the K nearest neighbors.
    ///
    /// # Arguments
    ///
    /// * `query` - The query vector.
    /// * `k` - The number of neighbors to return.
    /// * `storage` - The vector storage for distance calculations.
    ///
    /// # Returns
    ///
    /// A list of `SearchResult`s, sorted by distance (ascending).
    /// Returns at most `k` results, or fewer if:
    /// - The index has fewer than `k` live (non-deleted) vectors
    /// - Not enough neighbors were found during traversal
    ///
    /// # Tombstone Handling (v0.3.0)
    ///
    /// Search automatically filters out deleted vectors:
    ///
    /// 1. **Routing:** Deleted nodes are still traversed during graph navigation.
    ///    This preserves graph connectivity and search quality.
    ///
    /// 2. **Filtering:** Deleted nodes are excluded from the final results.
    ///    Only live vectors appear in the returned `SearchResult` list.
    ///
    /// 3. **Compensation:** The search internally over-fetches using `adjusted_k()`
    ///    to ensure `k` live results are returned even with tombstones.
    ///    At high tombstone ratios (>30%), consider calling `compact()`.
    ///
    /// # Thread Safety
    ///
    /// Per RFC-001, this method accepts eventual consistency. If `soft_delete()`
    /// is called concurrently (which requires `&mut self`), Rust's borrow checker
    /// prevents data races. The API is designed for single-threaded access.
    ///
    /// # Errors
    ///
    /// Returns `GraphError` if:
    /// - Query dimension mismatches index dimension
    /// - Config metric is invalid
    /// - Internal graph corruption occurs
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        storage: &VectorStorage,
    ) -> Result<Vec<SearchResult>, GraphError> {
        let mut search_ctx = SearchContext::new();
        self.search_with_context(query, k, storage, &mut search_ctx)
    }

    /// Searches the index for the K nearest neighbors with a reusable context.
    ///
    /// This method allows reusing allocations across multiple searches for better performance.
    ///
    /// # Arguments
    ///
    /// * `query` - The query vector.
    /// * `k` - The number of neighbors to return.
    /// * `storage` - The vector storage for distance calculations.
    /// * `ctx` - A reusable search context to avoid allocations.
    ///
    /// # Returns
    ///
    /// A list of `SearchResult`s, sorted by distance (ascending).
    ///
    /// # Errors
    ///
    /// Returns `GraphError` if:
    /// - Config metric is invalid.
    /// - Internal graph corruption occurs.
    ///
    /// # Performance
    ///
    /// Reusing `SearchContext` across searches can significantly improve performance
    /// by avoiding repeated allocations of HashSets and heaps. This is especially
    /// important for high-throughput scenarios or large-scale indexes (100k+ vectors).
    pub fn search_with_context(
        &self,
        query: &[f32],
        k: usize,
        storage: &VectorStorage,
        ctx: &mut SearchContext,
    ) -> Result<Vec<SearchResult>, GraphError> {
        if query.len() != self.config.dimensions as usize {
            return Err(GraphError::DimensionMismatch {
                expected: self.config.dimensions as usize,
                actual: query.len(),
            });
        }

        // Dispatch based on metric
        match self.config.metric {
            HnswConfig::METRIC_L2_SQUARED => self.search_impl::<L2Squared>(query, k, storage, ctx),
            HnswConfig::METRIC_DOT_PRODUCT | HnswConfig::METRIC_COSINE => {
                self.search_impl::<DotProduct>(query, k, storage, ctx)
            }
            HnswConfig::METRIC_HAMMING => {
                // For Hamming metric, we need to convert the f32 query to binary first
                let binary_query = crate::quantization::BinaryQuantizer::quantize_to_bytes(query);
                self.search_binary_impl(&binary_query, k, storage, ctx)
            }
            _ => Err(GraphError::InvalidConfig(format!(
                "unsupported metric code: {}",
                self.config.metric
            ))),
        }
    }

    fn search_impl<M: Metric<f32>>(
        &self,
        query: &[f32],
        k: usize,
        storage: &VectorStorage,
        search_ctx: &mut SearchContext,
    ) -> Result<Vec<SearchResult>, GraphError> {
        self.search_impl_with_ef::<M>(
            query,
            k,
            self.config.ef_search as usize,
            storage,
            search_ctx,
        )
    }

    fn search_impl_with_ef<M: Metric<f32>>(
        &self,
        query: &[f32],
        k: usize,
        ef_search: usize,
        storage: &VectorStorage,
        search_ctx: &mut SearchContext,
    ) -> Result<Vec<SearchResult>, GraphError> {
        let Some(entry_point) = self.entry_point() else {
            return Ok(Vec::new());
        };

        let mut curr_ep = entry_point;
        let max_layer = self.max_layer();

        // 1. Zoom down from max_layer to 1
        for lc in (1..=max_layer).rev() {
            let searcher = Searcher::<M, VectorStorage>::new(self, storage);
            searcher.search_layer(search_ctx, [curr_ep], query, 1, lc)?;
            if let Some(best) = search_ctx.scratch.first() {
                curr_ep = best.node_id;
            }
        }

        // 2. Search layer 0 with ef_search
        // W16.3: Use adjusted_k to compensate for tombstones
        let adjusted_k = self.adjusted_k(k);
        let ef = adjusted_k.max(ef_search);
        let searcher = Searcher::<M, VectorStorage>::new(self, storage);
        searcher.search_layer(search_ctx, [curr_ep], query, ef, 0)?;

        // 3. Extract top K, filtering out deleted vectors (W16.3)
        //
        // DEFENSE-IN-DEPTH (Intentional Redundancy):
        // search_layer() already filters deleted vectors during candidate collection.
        // This final check is intentionally redundant to provide a safety net:
        // - Protects against future refactoring that might bypass layer-level filtering
        // - Ensures correctness even if search_layer implementation changes
        // - Zero-cost when there are no tombstones (most common case)
        //
        // Per HOSTILE_REVIEWER m1: This redundancy is INTENTIONAL, not a bug.
        let mut results = Vec::with_capacity(k);
        for c in &search_ctx.scratch {
            if results.len() >= k {
                break;
            }
            let node = self
                .get_node(c.node_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            // Final filter: exclude deleted vectors
            if node.deleted == 0 {
                results.push(SearchResult {
                    vector_id: node.vector_id,
                    distance: c.distance,
                });
            }
        }

        Ok(results)
    }

    // =========================================================================
    // Binary Vector Search (Hamming Distance)
    // =========================================================================

    /// Searches the index for the K nearest neighbors using a binary query.
    ///
    /// This method is optimized for binary vectors using Hamming distance.
    ///
    /// # Arguments
    ///
    /// * `query` - The binary query vector (packed bytes).
    /// * `k` - The number of neighbors to return.
    /// * `storage` - The vector storage for distance calculations.
    ///
    /// # Returns
    ///
    /// A list of `SearchResult`s, sorted by Hamming distance (ascending).
    ///
    /// # Errors
    ///
    /// Returns `GraphError` if:
    /// - Query byte length doesn't match expected bytes for dimensions
    /// - Internal graph corruption occurs
    pub fn search_binary(
        &self,
        query: &[u8],
        k: usize,
        storage: &VectorStorage,
    ) -> Result<Vec<SearchResult>, GraphError> {
        let mut ctx = SearchContext::new();
        self.search_binary_with_context(query, k, storage, &mut ctx)
    }

    /// Searches the index using a binary query with a reusable context.
    ///
    /// # Arguments
    ///
    /// * `query` - The binary query vector (packed bytes).
    /// * `k` - The number of neighbors to return.
    /// * `storage` - The vector storage for distance calculations.
    /// * `ctx` - A reusable search context to avoid allocations.
    ///
    /// # Returns
    ///
    /// A list of `SearchResult`s, sorted by Hamming distance (ascending).
    pub fn search_binary_with_context(
        &self,
        query: &[u8],
        k: usize,
        storage: &VectorStorage,
        ctx: &mut SearchContext,
    ) -> Result<Vec<SearchResult>, GraphError> {
        // Validate dimensions: for binary, dimensions is in bits
        let expected_bytes = ((self.config.dimensions + 7) / 8) as usize;
        if query.len() != expected_bytes {
            return Err(GraphError::DimensionMismatch {
                expected: expected_bytes,
                actual: query.len(),
            });
        }

        self.search_binary_impl(query, k, storage, ctx)
    }

    /// Searches the index using a binary query with a custom ef_search parameter.
    ///
    /// This allows tuning the recall/speed tradeoff per-query without changing config.
    /// Higher ef_search = better recall, slower search.
    ///
    /// # Arguments
    ///
    /// * `query` - The binary query vector (packed bytes).
    /// * `k` - The number of neighbors to return.
    /// * `ef_search` - Size of dynamic candidate list (higher = better recall).
    /// * `storage` - The vector storage for distance calculations.
    ///
    /// # Returns
    ///
    /// A list of `SearchResult`s, sorted by Hamming distance (ascending).
    pub fn search_binary_with_ef(
        &self,
        query: &[u8],
        k: usize,
        ef_search: usize,
        storage: &VectorStorage,
    ) -> Result<Vec<SearchResult>, GraphError> {
        let mut ctx = SearchContext::new();
        self.search_binary_with_ef_context(query, k, ef_search, storage, &mut ctx)
    }

    /// Searches the index using a binary query with custom ef_search and reusable context.
    ///
    /// # Arguments
    ///
    /// * `query` - The binary query vector (packed bytes).
    /// * `k` - The number of neighbors to return.
    /// * `ef_search` - Size of dynamic candidate list (higher = better recall).
    /// * `storage` - The vector storage for distance calculations.
    /// * `ctx` - A reusable search context to avoid allocations.
    pub fn search_binary_with_ef_context(
        &self,
        query: &[u8],
        k: usize,
        ef_search: usize,
        storage: &VectorStorage,
        ctx: &mut SearchContext,
    ) -> Result<Vec<SearchResult>, GraphError> {
        // Validate dimensions: for binary, dimensions is in bits
        let expected_bytes = ((self.config.dimensions + 7) / 8) as usize;
        if query.len() != expected_bytes {
            return Err(GraphError::DimensionMismatch {
                expected: expected_bytes,
                actual: query.len(),
            });
        }

        self.search_binary_impl_with_ef(query, k, ef_search, storage, ctx)
    }

    fn search_binary_impl(
        &self,
        query: &[u8],
        k: usize,
        storage: &VectorStorage,
        search_ctx: &mut SearchContext,
    ) -> Result<Vec<SearchResult>, GraphError> {
        self.search_binary_impl_with_ef(
            query,
            k,
            self.config.ef_search as usize,
            storage,
            search_ctx,
        )
    }

    fn search_binary_impl_with_ef(
        &self,
        query: &[u8],
        k: usize,
        ef_search: usize,
        storage: &VectorStorage,
        search_ctx: &mut SearchContext,
    ) -> Result<Vec<SearchResult>, GraphError> {
        // Guard: binary search requires Hamming metric
        if self.config.metric != HnswConfig::METRIC_HAMMING {
            return Err(GraphError::InvalidConfig(format!(
                "search_binary requires metric=Hamming, got {}",
                self.config.metric
            )));
        }

        let Some(entry_point) = self.entry_point() else {
            return Ok(Vec::new());
        };

        let mut curr_ep = entry_point;
        let max_layer = self.max_layer();

        // 1. Zoom down from max_layer to 1
        for lc in (1..=max_layer).rev() {
            self.search_binary_layer(search_ctx, [curr_ep], query, 1, lc, storage)?;
            if let Some(best) = search_ctx.scratch.first() {
                curr_ep = best.node_id;
            }
        }

        // 2. Search layer 0 with ef_search
        let adjusted_k = self.adjusted_k(k);
        let ef = adjusted_k.max(ef_search);
        self.search_binary_layer(search_ctx, [curr_ep], query, ef, 0, storage)?;

        // 3. Extract top K, filtering out deleted vectors
        let mut results = Vec::with_capacity(k);
        for c in &search_ctx.scratch {
            if results.len() >= k {
                break;
            }
            let node = self
                .get_node(c.node_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            if node.deleted == 0 {
                results.push(SearchResult {
                    vector_id: node.vector_id,
                    distance: c.distance,
                });
            }
        }

        Ok(results)
    }

    /// Performs a greedy search on a specific layer using binary vectors.
    #[inline]
    pub(crate) fn search_binary_layer(
        &self,
        ctx: &mut SearchContext,
        entry_points: impl IntoIterator<Item = NodeId>,
        query: &[u8],
        ef: usize,
        level: u8,
        storage: &VectorStorage,
    ) -> Result<(), GraphError> {
        ctx.clear();

        // 1. Initialize
        for ep in entry_points {
            let node = self.get_node(ep).ok_or(GraphError::NodeIdOutOfBounds)?;
            if node.max_layer < level {
                continue;
            }

            // Binary storage: get binary vector directly
            let stored = storage.get_binary_vector(node.vector_id);
            let dist = Hamming::distance(query, stored);

            let candidate = Candidate {
                distance: dist,
                node_id: ep,
            };

            ctx.candidates.push(Reverse(candidate));
            if node.deleted == 0 {
                ctx.results.push(candidate);
            }
            ctx.visited.insert(ep);
        }

        // Prune initial
        while ctx.results.len() > ef {
            ctx.results.pop();
        }

        let traversal_limit = ef.saturating_mul(MAX_TRAVERSAL_MULT);
        let mut traversed_count = 0;

        // 2. Greedy Search
        while let Some(Reverse(candidate)) = ctx.candidates.pop() {
            traversed_count += 1;
            if traversed_count > traversal_limit {
                break;
            }

            if let Some(furthest) = ctx.results.peek() {
                if candidate.distance > furthest.distance && ctx.results.len() >= ef {
                    break;
                }
            }

            let node = self
                .get_node(candidate.node_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;

            let start = node.neighbor_offset as usize;
            let end = start + node.neighbor_len as usize;

            if end > self.neighbors.buffer.len() {
                return Err(GraphError::NeighborError);
            }

            let slice = &self.neighbors.buffer[start..end];
            let neighbor_iter = crate::hnsw::neighbor::NeighborPool::iter_layer(slice, level);

            for neighbor_id_u32 in neighbor_iter {
                let neighbor_id = NodeId(neighbor_id_u32);
                if !ctx.visited.contains(&neighbor_id) {
                    ctx.visited.insert(neighbor_id);

                    let neighbor_node = self
                        .get_node(neighbor_id)
                        .ok_or(GraphError::NodeIdOutOfBounds)?;

                    // Binary storage: get binary vector directly
                    let stored = storage.get_binary_vector(neighbor_node.vector_id);
                    let dist = Hamming::distance(query, stored);

                    let mut should_add = false;
                    if ctx.results.len() < ef {
                        should_add = true;
                    } else if let Some(furthest) = ctx.results.peek() {
                        if dist < furthest.distance {
                            should_add = true;
                        }
                    }

                    if should_add {
                        let new_candidate = Candidate {
                            distance: dist,
                            node_id: neighbor_id,
                        };
                        ctx.candidates.push(Reverse(new_candidate));

                        if neighbor_node.deleted == 0 {
                            ctx.results.push(new_candidate);

                            if ctx.results.len() > ef {
                                ctx.results.pop();
                            }
                        }
                    }
                }
            }
        }

        // Return sorted results
        while let Some(c) = ctx.results.pop() {
            ctx.scratch.push(c);
        }
        ctx.scratch.reverse();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hnsw::config::HnswConfig;
    use crate::hnsw::graph::HnswIndex;
    use crate::storage::VectorStorage;

    #[test]
    fn test_candidate_ordering() {
        let c1 = Candidate {
            distance: 1.0,
            node_id: NodeId(1),
        };
        let c2 = Candidate {
            distance: 2.0,
            node_id: NodeId(2),
        };
        assert!(c1 < c2);
    }

    #[test]
    fn test_search_safety_limit() {
        let dim = 4;
        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

        // 2. Create a chain of nodes: 0 -> 1 -> 2 ... -> 19
        // Vectors: [i, 0, 0, 0]. Query: [100, 0, 0, 0].
        // Distance decreases as i increases.
        let chain_len = 20;
        let mut node_ids = Vec::new();

        #[allow(clippy::cast_precision_loss)]
        for i in 0..chain_len {
            let vec = vec![i as f32, 0.0, 0.0, 0.0];
            let vid = storage.insert(&vec).unwrap();
            let nid = index.add_node(vid, 0).unwrap();
            node_ids.push(nid);
        }

        // Link them: i -> i+1
        for i in 0..chain_len - 1 {
            index
                .set_neighbors(node_ids[i], &[node_ids[i + 1]])
                .unwrap();
        }

        // 3. Search
        let query = vec![100.0, 0.0, 0.0, 0.0];
        let mut ctx = SearchContext::new();
        // ef=1 => Limit = 1 * 10 = 10.
        // If we traverse more than 10 nodes, we should stop.
        let ef = 1;

        let searcher = Searcher::<crate::metric::L2Squared, VectorStorage>::new(&index, &storage);

        // Start at node 0.
        // It will greedily follow the chain 0 -> 1 -> 2 ...
        // We expect it to stop around node 9 or 10.
        searcher
            .search_layer(&mut ctx, [node_ids[0]], &query, ef, 0)
            .unwrap();

        // 4. Verification
        // With limit=10, we should have traversed 11 nodes (pop count).
        // 1. Pop 0 (count=1). Expand 0. Add 1.
        // ...
        // 10. Pop 9 (count=10). Expand 9. Add 10.
        // 11. Pop 10 (count=11). > 10 => Break.
        // Node 10 is in visited, but not expanded.
        // Node 11 is NOT in visited.
        // Total visited: 0..10 -> 11 nodes.

        let visited_ids: HashSet<u32> = ctx.visited.iter().map(|n| n.0).collect();
        assert_eq!(
            visited_ids.len(),
            11,
            "Should have visited exactly 11 nodes"
        );

        assert!(ctx.visited.contains(&node_ids[10])); // Node 10 is in visited
        assert!(!ctx.visited.contains(&node_ids[11])); // Node 11 never seen
    }

    #[test]
    fn test_search_dimension_mismatch() {
        let dim = 128;
        let config = HnswConfig::new(dim);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::new(config, &storage).unwrap();

        let query = vec![0.0; dim as usize + 1]; // Wrong dimension
        let result = index.search(&query, 10, &storage);

        assert!(matches!(
            result,
            Err(GraphError::DimensionMismatch {
                expected: 128,
                actual: 129
            })
        ));
    }
}

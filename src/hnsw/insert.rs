use super::config::HnswConfig;
use super::graph::{GraphError, HnswIndex, NodeId, VectorId, VectorProvider};
use super::search::{Candidate, SearchContext, Searcher};
use crate::hnsw::neighbor::NeighborPool;
use crate::metric::simd::l2_squared_u8;
use crate::metric::{DotProduct, Hamming, L2Squared, Metric};
use crate::quantization::variable::BinaryVector;
use crate::storage::VectorStorage;

// Helper: Skip VByte encoded integer
fn vbyte_decode_skip(data: &[u8], cursor: usize) -> (u32, usize) {
    let mut val = 0u32;
    let mut shift = 0;
    let mut bytes_read = 0;
    for byte in data.iter().skip(cursor) {
        bytes_read += 1;
        val |= u32::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return (val, bytes_read);
        }
        shift += 7;
        // Overflow protection matching neighbor.rs
        if shift >= 35 {
            return (val, bytes_read);
        }
    }
    (val, bytes_read)
}

// Helper: Scan blob to find start and end of a specific layer
// Returns (start_offset, end_offset, current_level_count)
fn scan_blob_for_layer(blob: &[u8], target_layer: u8) -> (usize, usize, u8) {
    let mut cursor = 0;
    let mut current_level = 0u8;

    while cursor < blob.len() {
        if current_level == target_layer {
            let start = cursor;
            // Decode count
            let (count, bytes) = vbyte_decode_skip(blob, cursor);
            cursor += bytes;
            // Skip count * deltas
            for _ in 0..count {
                if cursor >= blob.len() {
                    break;
                }
                let (_, b) = vbyte_decode_skip(blob, cursor);
                cursor += b;
            }
            return (start, cursor, current_level);
        }

        // Skip this level
        let (count, bytes) = vbyte_decode_skip(blob, cursor);
        cursor += bytes;
        for _ in 0..count {
            if cursor >= blob.len() {
                break;
            }
            let (_, b) = vbyte_decode_skip(blob, cursor);
            cursor += b;
        }
        current_level += 1;
    }

    // Not found (target_layer >= layers in blob)
    (cursor, cursor, current_level)
}

impl HnswIndex {
    /// Inserts a vector into the index.
    ///
    /// This method performs the full HNSW insertion algorithm:
    /// 1. Stores the vector in `VectorStorage`.
    /// 2. Assigns a new `NodeId` and determines its level `L`.
    /// 3. Finds the entry point at layer `L`.
    /// 4. Connects the new node to neighbors from layer `L` down to 0.
    ///
    /// # Arguments
    ///
    /// * `vector` - The raw vector data (must match configured dimensions).
    /// * `storage` - The storage backend (must match configuration).
    ///
    /// # Returns
    ///
    /// The assigned `VectorId` on success, or a `GraphError` on failure.
    ///
    /// # Errors
    ///
    /// Returns `GraphError` if:
    /// - Storage dimensions mismatch.
    /// - Config metric is invalid.
    /// - Internal graph corruption occurs.
    pub fn insert(
        &mut self,
        vector: &[f32],
        storage: &mut VectorStorage,
    ) -> Result<VectorId, GraphError> {
        // Dispatch based on metric
        match self.config.metric {
            HnswConfig::METRIC_L2_SQUARED => self.insert_impl::<L2Squared>(vector, storage),
            HnswConfig::METRIC_DOT_PRODUCT | HnswConfig::METRIC_COSINE => {
                self.insert_impl::<DotProduct>(vector, storage)
            }
            HnswConfig::METRIC_HAMMING => {
                // For Hamming metric, auto-convert f32 to binary and insert
                let binary = crate::quantization::BinaryQuantizer::quantize_to_bytes(vector);
                self.insert_binary_impl(&binary, storage)
            }
            _ => Err(GraphError::InvalidConfig(format!(
                "unsupported metric code: {}",
                self.config.metric
            ))),
        }
    }

    /// Inserts a vector with automatic binary quantization (v0.7.0 - RFC-002 Phase 2).
    ///
    /// If BQ is enabled, the vector is stored in both F32 and BQ format.
    /// If BQ is disabled, this behaves identically to `insert()`.
    ///
    /// # Arguments
    ///
    /// * `vector` - The vector to insert.
    /// * `storage` - The F32 vector storage.
    ///
    /// # Returns
    ///
    /// The assigned vector ID.
    ///
    /// # Errors
    ///
    /// - `GraphError::DimensionMismatch` if vector dimension is wrong.
    /// - `GraphError::Quantization` if BQ quantization fails.
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
    /// let id = index.insert_bq(&v, &mut storage).unwrap();
    ///
    /// assert!(index.has_bq());
    /// assert_eq!(index.bq_storage().unwrap().len(), 1);
    /// ```
    pub fn insert_bq(
        &mut self,
        vector: &[f32],
        storage: &mut VectorStorage,
    ) -> Result<VectorId, GraphError> {
        // Step 1: Validate dimension
        let expected_dim = self.config.dimensions as usize;
        if vector.len() != expected_dim {
            return Err(GraphError::DimensionMismatch {
                expected: expected_dim,
                actual: vector.len(),
            });
        }

        // Step 2: Insert into F32 storage and HNSW graph
        // Note: insert_impl now automatically handles BQ insertion when enabled,
        // so no manual BQ insertion is needed here anymore.
        self.insert(vector, storage)
    }

    /// Generic implementation of insert for a specific metric.
    fn insert_impl<M: Metric<f32>>(
        &mut self,
        vector: &[f32],
        storage: &mut VectorStorage,
    ) -> Result<VectorId, GraphError> {
        // Step 1: Store vector in VectorStorage
        let vector_id = storage
            .insert(vector)
            .map_err(|e| GraphError::Storage(e.to_string()))?;

        // Step 2: Determine random level L
        let level = self.get_random_level();

        // Add node to graph
        let new_node_id = self.add_node(vector_id, level)?;

        // Step 3: Phase 1 - Greedy search to find entry point at level L+1
        let mut ep = self.entry_point();

        let mut search_ctx = SearchContext::new();

        if let Some(entry_point_id) = ep {
            let entry_node = self
                .get_node(entry_point_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            let entry_max_layer = entry_node.max_layer;

            let mut curr_ep = entry_point_id;

            // Go down from top to L+1
            // Only if entry node actually goes that high
            for lc in (level + 1..=entry_max_layer).rev() {
                // Search layer with ef=1 for greedy traversal
                let searcher = Searcher::<M, VectorStorage>::new(self, storage);
                searcher.search_layer(&mut search_ctx, [curr_ep], vector, 1, lc)?;

                if let Some(best) = search_ctx.scratch.first() {
                    curr_ep = best.node_id;
                }
            }
            ep = Some(curr_ep);
        }

        // Step 4: Phase 2 - Insert from min(L, max_layer) down to 0
        let start_layer = if let Some(entry_point_id) = self.entry_point() {
            let entry_node = self
                .get_node(entry_point_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            std::cmp::min(level, entry_node.max_layer)
        } else {
            0
        };

        if let Some(mut curr_ep) = ep {
            for lc in (0..=start_layer).rev() {
                let ef = self.config.ef_construction as usize;

                // 1. Search for candidates
                let searcher = Searcher::<M, VectorStorage>::new(self, storage);
                searcher.search_layer(&mut search_ctx, [curr_ep], vector, ef, lc)?;

                // Save best candidate for next iteration (before scratch is clobbered)
                let next_ep = search_ctx.scratch.first().map(|c| c.node_id);

                // 2. Select M neighbors
                let m_max = if lc == 0 {
                    self.config.m0
                } else {
                    self.config.m
                } as usize;

                {
                    // Scope for split borrow
                    let SearchContext {
                        ref scratch,
                        ref mut neighbor_scratch,
                        ..
                    } = search_ctx;

                    self.select_neighbors_heuristic::<M>(
                        vector,
                        scratch,
                        m_max,
                        lc,
                        storage,
                        neighbor_scratch,
                    )?;
                } // Drop split borrow

                // 3. Connect Bidirectionally
                // We clone neighbors to avoid borrowing search_ctx
                // This allocation (Vec<NodeId>) is small (size M) and unavoidable
                // unless we use a second persistent scratch buffer or stack array.
                // For now, C1 optimization is "no large allocations". This is small.
                let neighbors = search_ctx.neighbor_scratch.clone();

                for &neighbor_id in &neighbors {
                    // New -> Neighbor
                    self.add_connection::<M>(
                        new_node_id,
                        neighbor_id,
                        lc,
                        storage,
                        &mut search_ctx,
                    )?;

                    // Neighbor -> New
                    self.add_connection::<M>(
                        neighbor_id,
                        new_node_id,
                        lc,
                        storage,
                        &mut search_ctx,
                    )?;
                }

                // 4. Update entry point for next layer
                if let Some(best_id) = next_ep {
                    curr_ep = best_id;
                }
            }
        }

        // Step 5: Update global entry point if needed
        if self.entry_point().is_none() || level > start_layer {
            self.set_entry_point(new_node_id);
        }

        // Step 6: If BQ is enabled, quantize and insert into BQ storage
        // This ensures all insert methods (insert, insert_with_metadata, batch_insert)
        // automatically maintain BQ storage consistency.
        if let Some(ref mut bq_storage) = self.bq_storage {
            let bv = BinaryVector::quantize(vector)
                .map_err(|e| GraphError::Quantization(e.to_string()))?;
            bq_storage
                .insert(&bv)
                .map_err(|e| GraphError::Storage(e.to_string()))?;
        }

        Ok(vector_id)
    }

    // =========================================================================
    // Binary Vector Insert (Hamming Distance)
    // =========================================================================

    /// Inserts a pre-packed binary vector into the index.
    ///
    /// This method is for binary vectors (1-bit quantized) using Hamming distance.
    /// Use this when you have pre-quantized data (e.g., from Turso's `f1bit_blob`).
    ///
    /// # Arguments
    ///
    /// * `binary` - The binary vector (packed bytes). Length must equal `ceil(dimensions / 8)`.
    /// * `storage` - The storage backend (must be in Binary mode).
    ///
    /// # Returns
    ///
    /// The assigned `VectorId` on success, or a `GraphError` on failure.
    ///
    /// # Errors
    ///
    /// Returns `GraphError` if:
    /// - Storage is not in Binary mode.
    /// - Binary vector length doesn't match expected bytes.
    /// - Internal graph corruption occurs.
    pub fn insert_binary(
        &mut self,
        binary: &[u8],
        storage: &mut VectorStorage,
    ) -> Result<VectorId, GraphError> {
        // Validate dimensions
        let expected_bytes = ((self.config.dimensions + 7) / 8) as usize;
        if binary.len() != expected_bytes {
            return Err(GraphError::DimensionMismatch {
                expected: expected_bytes,
                actual: binary.len(),
            });
        }

        self.insert_binary_impl(binary, storage)
    }

    /// Inserts an f32 vector with automatic binary quantization.
    ///
    /// The vector is converted to binary (1 bit per dimension) using sign quantization:
    /// - Positive values → 1
    /// - Non-positive values → 0
    ///
    /// # Arguments
    ///
    /// * `vector` - The f32 vector (must match configured dimensions).
    /// * `storage` - The storage backend (must be in Binary mode).
    ///
    /// # Returns
    ///
    /// The assigned `VectorId` on success, or a `GraphError` on failure.
    pub fn insert_with_bq(
        &mut self,
        vector: &[f32],
        storage: &mut VectorStorage,
    ) -> Result<VectorId, GraphError> {
        // Validate dimensions
        if vector.len() != self.config.dimensions as usize {
            return Err(GraphError::DimensionMismatch {
                expected: self.config.dimensions as usize,
                actual: vector.len(),
            });
        }

        // Convert to binary using binary quantizer
        let binary = crate::quantization::BinaryQuantizer::quantize_to_bytes(vector);
        self.insert_binary_impl(&binary, storage)
    }

    /// Implementation for binary vector insertion.
    fn insert_binary_impl(
        &mut self,
        binary: &[u8],
        storage: &mut VectorStorage,
    ) -> Result<VectorId, GraphError> {
        // Step 1: Store binary vector in VectorStorage
        let vector_id = storage
            .insert_binary(binary)
            .map_err(|e| GraphError::Storage(e.to_string()))?;

        // Step 2: Determine random level L
        let level = self.get_random_level();

        // Add node to graph
        let new_node_id = self.add_node(vector_id, level)?;

        // Step 3: Phase 1 - Greedy search to find entry point at level L+1
        let mut ep = self.entry_point();

        let mut search_ctx = SearchContext::new();

        if let Some(entry_point_id) = ep {
            let entry_node = self
                .get_node(entry_point_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            let entry_max_layer = entry_node.max_layer;

            let mut curr_ep = entry_point_id;

            // Go down from top to L+1
            for lc in (level + 1..=entry_max_layer).rev() {
                self.search_binary_layer(&mut search_ctx, [curr_ep], binary, 1, lc, storage)?;

                if let Some(best) = search_ctx.scratch.first() {
                    curr_ep = best.node_id;
                }
            }
            ep = Some(curr_ep);
        }

        // Step 4: Phase 2 - Insert from min(L, max_layer) down to 0
        let start_layer = if let Some(entry_point_id) = self.entry_point() {
            let entry_node = self
                .get_node(entry_point_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            std::cmp::min(level, entry_node.max_layer)
        } else {
            0
        };

        if let Some(mut curr_ep) = ep {
            for lc in (0..=start_layer).rev() {
                let ef = self.config.ef_construction as usize;

                // 1. Search for candidates
                self.search_binary_layer(&mut search_ctx, [curr_ep], binary, ef, lc, storage)?;

                // Save best candidate for next iteration
                let next_ep = search_ctx.scratch.first().map(|c| c.node_id);

                // 2. Select M neighbors
                let m_max = if lc == 0 {
                    self.config.m0
                } else {
                    self.config.m
                } as usize;

                {
                    let SearchContext {
                        ref scratch,
                        ref mut neighbor_scratch,
                        ..
                    } = search_ctx;

                    self.select_neighbors_heuristic_binary(
                        binary,
                        scratch,
                        m_max,
                        storage,
                        neighbor_scratch,
                    )?;
                }

                // 3. Connect Bidirectionally
                let neighbors = search_ctx.neighbor_scratch.clone();

                for &neighbor_id in &neighbors {
                    // New -> Neighbor
                    self.add_connection_binary(
                        new_node_id,
                        neighbor_id,
                        lc,
                        storage,
                        &mut search_ctx,
                    )?;

                    // Neighbor -> New
                    self.add_connection_binary(
                        neighbor_id,
                        new_node_id,
                        lc,
                        storage,
                        &mut search_ctx,
                    )?;
                }

                // 4. Update entry point for next layer
                if let Some(best_id) = next_ep {
                    curr_ep = best_id;
                }
            }
        }

        // Step 5: Update global entry point if needed
        if self.entry_point().is_none() || level > start_layer {
            self.set_entry_point(new_node_id);
        }

        Ok(vector_id)
    }

    /// Helper: Select neighbors using HNSW heuristic for binary vectors.
    fn select_neighbors_heuristic_binary(
        &self,
        _query: &[u8],
        candidates: &[Candidate],
        m: usize,
        storage: &VectorStorage,
        output: &mut Vec<NodeId>,
    ) -> Result<(), GraphError> {
        output.clear();
        if candidates.is_empty() {
            return Ok(());
        }

        for c in candidates {
            if output.len() >= m {
                break;
            }

            let c_node = self
                .get_node(c.node_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            let dist_q_c = c.distance;

            let mut closer_to_existing = false;

            // Get binary vector for candidate
            let c_bin = storage
                .get_binary_vector(c_node.vector_id)
                .map_err(|e| GraphError::Storage(e.to_string()))?;

            for &r_id in output.iter() {
                let r_node = self.get_node(r_id).ok_or(GraphError::NodeIdOutOfBounds)?;
                let r_bin = storage
                    .get_binary_vector(r_node.vector_id)
                    .map_err(|e| GraphError::Storage(e.to_string()))?;

                let dist_c_r = Hamming::distance(c_bin, r_bin);

                if dist_c_r < dist_q_c {
                    closer_to_existing = true;
                    break;
                }
            }

            if !closer_to_existing {
                output.push(c.node_id);
            }
        }

        Ok(())
    }

    /// Helper: Add a connection between two nodes for binary vectors.
    fn add_connection_binary(
        &mut self,
        source: NodeId,
        target: NodeId,
        layer: u8,
        storage: &VectorStorage,
        ctx: &mut SearchContext,
    ) -> Result<(), GraphError> {
        // 1. Read source node info
        let (max_layer, vector_id) = {
            let node = self.get_node(source).ok_or(GraphError::NodeIdOutOfBounds)?;
            (node.max_layer, node.vector_id)
        };

        if layer > max_layer {
            return Ok(());
        }

        let node_idx = source.0 as usize;
        let (old_offset, old_len) = {
            let node = &self.nodes[node_idx];
            (node.neighbor_offset, node.neighbor_len)
        };

        // 2. Scan to find the target layer range and decode neighbors
        ctx.neighbor_id_scratch.clear();

        let start;
        let end;
        let max_level_found;
        let valid_end;

        {
            let blob = &self.neighbors.buffer
                [old_offset as usize..(old_offset as usize + old_len as usize)];
            let (s, e, m) = scan_blob_for_layer(blob, layer);
            start = s;
            end = e;
            max_level_found = m;

            valid_end = if layer == max_layer {
                e
            } else {
                let (_, ve, _) = scan_blob_for_layer(blob, max_layer);
                ve
            };

            if s < e {
                NeighborPool::decode_neighbors_to_buf(&blob[s..e], &mut ctx.neighbor_id_scratch);
            }
        }

        // Check duplicates
        if ctx.neighbor_id_scratch.contains(&target.0) {
            return Ok(());
        }
        ctx.neighbor_id_scratch.push(target.0);

        // 3. Prune if needed
        let m_max = if layer == 0 {
            self.config.m0
        } else {
            self.config.m
        } as usize;

        if ctx.neighbor_id_scratch.len() > m_max {
            // Get source binary vector
            let source_bin = storage
                .get_binary_vector(vector_id)
                .map_err(|e| GraphError::Storage(e.to_string()))?;

            ctx.scratch.clear();

            for &n_u32 in &ctx.neighbor_id_scratch {
                let n_id = NodeId(n_u32);
                let n_node = self.get_node(n_id).ok_or(GraphError::NodeIdOutOfBounds)?;
                let n_bin = storage
                    .get_binary_vector(n_node.vector_id)
                    .map_err(|e| GraphError::Storage(e.to_string()))?;
                let dist = Hamming::distance(source_bin, n_bin);
                ctx.scratch.push(Candidate {
                    distance: dist,
                    node_id: n_id,
                });
            }

            ctx.scratch
                .sort_by(|a, b| a.distance.total_cmp(&b.distance));

            self.select_neighbors_heuristic_binary(
                source_bin,
                &ctx.scratch,
                m_max,
                storage,
                &mut ctx.neighbor_scratch,
            )?;

            ctx.neighbor_id_scratch.clear();
            ctx.neighbor_id_scratch
                .extend(ctx.neighbor_scratch.iter().map(|n| n.0));
        }

        // 4. Encode modified layer
        ctx.encoding_scratch.clear();
        NeighborPool::encode_neighbors_to_buf(&ctx.neighbor_id_scratch, &mut ctx.encoding_scratch);
        let encoded_new_layer = &ctx.encoding_scratch;

        // 5. Calculate new size and fill gaps
        let gap_count = if layer > max_level_found {
            (layer - max_level_found) as usize
        } else {
            0
        };

        let new_size = start + gap_count + encoded_new_layer.len() + (valid_end - end);

        // 6. Alloc new space
        let (new_offset, new_capacity) = self.neighbors.alloc(new_size)?;

        let new_offset_u = new_offset as usize;
        let old_offset_u = old_offset as usize;

        // 7. Write parts
        if start > 0 {
            self.neighbors
                .buffer
                .copy_within(old_offset_u..old_offset_u + start, new_offset_u);
        }

        let mut curr = new_offset_u + start;

        for _ in 0..gap_count {
            self.neighbors.buffer[curr] = 0;
            curr += 1;
        }

        self.neighbors.buffer[curr..curr + encoded_new_layer.len()]
            .copy_from_slice(encoded_new_layer);
        curr += encoded_new_layer.len();

        let remaining = valid_end.saturating_sub(end);
        if remaining > 0 {
            let src_start = old_offset_u + end;
            self.neighbors
                .buffer
                .copy_within(src_start..src_start + remaining, curr);
            curr += remaining;
        }

        let allocated_end = new_offset_u + new_capacity as usize;
        if curr < allocated_end {
            self.neighbors.buffer[curr..allocated_end].fill(0);
        }

        // 8. Update node and free old memory
        let node = &mut self.nodes[node_idx];
        if old_len > 0 {
            self.neighbors.free(old_offset, old_len);
        }
        node.neighbor_offset = new_offset;
        node.neighbor_len = new_capacity;

        Ok(())
    }

    /// Helper: Select neighbors using HNSW heuristic.
    fn select_neighbors_heuristic<M: Metric<f32>>(
        &self,
        _query: &[f32],
        candidates: &[Candidate],
        m: usize,
        _layer: u8,
        storage: &VectorStorage,
        output: &mut Vec<NodeId>,
    ) -> Result<(), GraphError> {
        output.clear();
        if candidates.is_empty() {
            return Ok(());
        }

        // Check if we should use quantized distance
        // Logic matches search_layer in search.rs
        let use_quantized = if self.config.metric == HnswConfig::METRIC_L2_SQUARED {
            // Use VectorProvider trait method to check if quantized data is available
            // storage is &mut VectorStorage, which implements VectorProvider
            VectorProvider::get_quantized_vector(storage, VectorId::FIRST).is_some()
        } else {
            false
        };

        for c in candidates {
            if output.len() >= m {
                break;
            }

            // HNSW Heuristic
            let c_node = self
                .get_node(c.node_id)
                .ok_or(GraphError::NodeIdOutOfBounds)?;
            let dist_q_c = c.distance;

            let mut closer_to_existing = false;

            if use_quantized {
                // Use U8 distance
                let c_q_vec = VectorProvider::get_quantized_vector(storage, c_node.vector_id)
                    .ok_or_else(|| GraphError::Storage("Missing quantized data".into()))?;

                for &r_id in output.iter() {
                    let r_node = self.get_node(r_id).ok_or(GraphError::NodeIdOutOfBounds)?;
                    let r_q_vec =
                        VectorProvider::get_quantized_vector(storage, r_node.vector_id)
                            .ok_or_else(|| GraphError::Storage("Missing quantized data".into()))?;

                    // SAFETY: u32 -> f32 precision loss is acceptable for distance comparison.
                    // Max u32 is ~4e9, well within f32's representation range (~3.4e38).
                    #[allow(clippy::cast_precision_loss)]
                    let dist_c_r = l2_squared_u8(c_q_vec, r_q_vec) as f32;

                    if dist_c_r < dist_q_c {
                        closer_to_existing = true;
                        break;
                    }
                }
            } else {
                // Use F32 distance
                let c_vec = storage.get_vector(c_node.vector_id);
                for &r_id in output.iter() {
                    let r_node = self.get_node(r_id).ok_or(GraphError::NodeIdOutOfBounds)?;
                    let r_vec = storage.get_vector(r_node.vector_id);
                    let dist_c_r = M::distance(&c_vec, &r_vec);

                    if dist_c_r < dist_q_c {
                        closer_to_existing = true;
                        break;
                    }
                }
            }

            if !closer_to_existing {
                output.push(c.node_id);
            }
        }

        Ok(())
    }

    /// Helper: Add a connection between two nodes on a specific layer.
    ///
    /// Optimizes allocations by reusing SearchContext scratch buffers.
    fn add_connection<M: Metric<f32>>(
        &mut self,
        source: NodeId,
        target: NodeId,
        layer: u8,
        storage: &VectorStorage,
        ctx: &mut SearchContext,
    ) -> Result<(), GraphError> {
        // 1. Read source node info
        let (max_layer, vector_id) = {
            let node = self.get_node(source).ok_or(GraphError::NodeIdOutOfBounds)?;
            (node.max_layer, node.vector_id)
        };

        if layer > max_layer {
            return Ok(());
        }

        let node_idx = source.0 as usize;
        let (old_offset, old_len) = {
            let node = &self.nodes[node_idx];
            (node.neighbor_offset, node.neighbor_len)
        };

        // 2. Scan to find the target layer range and decode neighbors
        // We reuse ctx.neighbor_id_scratch for the neighbor list
        ctx.neighbor_id_scratch.clear();

        // We need to keep some layout info
        let start;
        let end;
        let max_level_found;
        let valid_end;

        {
            let blob = &self.neighbors.buffer
                [old_offset as usize..(old_offset as usize + old_len as usize)];
            let (s, e, m) = scan_blob_for_layer(blob, layer);
            start = s;
            end = e;
            max_level_found = m;

            // Find the end of valid data
            valid_end = if layer == max_layer {
                e
            } else {
                let (_, ve, _) = scan_blob_for_layer(blob, max_layer);
                ve
            };

            if s < e {
                NeighborPool::decode_neighbors_to_buf(&blob[s..e], &mut ctx.neighbor_id_scratch);
            }
        }

        // Check duplicates
        if ctx.neighbor_id_scratch.contains(&target.0) {
            return Ok(());
        }
        ctx.neighbor_id_scratch.push(target.0);

        // 3. Prune if needed
        let m_max = if layer == 0 {
            self.config.m0
        } else {
            self.config.m
        } as usize;

        if ctx.neighbor_id_scratch.len() > m_max {
            // Check if we should use quantized distance
            let use_quantized = if self.config.metric == HnswConfig::METRIC_L2_SQUARED {
                VectorProvider::get_quantized_vector(storage, VectorId::FIRST).is_some()
            } else {
                false
            };

            // Use scratch buffer for candidates
            ctx.scratch.clear();

            if use_quantized {
                // We need to quantize source vector first if not already available.
                // source_vec is not available in U8 directly here?
                // Wait, source is an existing node (we are adding connection FROM source).
                // So we can get it from storage.
                let source_q_vec = VectorProvider::get_quantized_vector(storage, vector_id)
                    .ok_or_else(|| GraphError::Storage("Missing quantized data".into()))?;

                for &n_u32 in &ctx.neighbor_id_scratch {
                    let n_id = NodeId(n_u32);
                    let n_node = self.get_node(n_id).ok_or(GraphError::NodeIdOutOfBounds)?;
                    let n_q_vec =
                        VectorProvider::get_quantized_vector(storage, n_node.vector_id)
                            .ok_or_else(|| GraphError::Storage("Missing quantized data".into()))?;

                    // SAFETY: u32 -> f32 precision loss is acceptable for distance comparison.
                    #[allow(clippy::cast_precision_loss)]
                    let dist = l2_squared_u8(source_q_vec, n_q_vec) as f32;
                    ctx.scratch.push(Candidate {
                        distance: dist,
                        node_id: n_id,
                    });
                }
            } else {
                let source_vec = storage.get_vector(vector_id);
                for &n_u32 in &ctx.neighbor_id_scratch {
                    let n_id = NodeId(n_u32);
                    let n_node = self.get_node(n_id).ok_or(GraphError::NodeIdOutOfBounds)?;
                    let n_vec = storage.get_vector(n_node.vector_id);
                    let dist = M::distance(&source_vec, &n_vec);
                    ctx.scratch.push(Candidate {
                        distance: dist,
                        node_id: n_id,
                    });
                }
            }

            ctx.scratch
                .sort_by(|a, b| a.distance.total_cmp(&b.distance));

            // Use neighbor_scratch for selection output
            // This is safe because select_neighbors_heuristic clears it first

            // Prepare source vector slice
            let dummy: &[f32] = &[];
            // We need to extend the lifetime of the Cow return from get_vector
            let source_cow;
            let source_slice = if use_quantized {
                dummy
            } else {
                source_cow = storage.get_vector(vector_id);
                &source_cow
            };

            self.select_neighbors_heuristic::<M>(
                source_slice,
                &ctx.scratch,
                m_max,
                layer,
                storage,
                &mut ctx.neighbor_scratch,
            )?;

            // Update neighbor_id_scratch with pruned list
            ctx.neighbor_id_scratch.clear();
            ctx.neighbor_id_scratch
                .extend(ctx.neighbor_scratch.iter().map(|n| n.0));
        }

        // 4. Encode modified layer using scratch buffer
        ctx.encoding_scratch.clear();
        NeighborPool::encode_neighbors_to_buf(&ctx.neighbor_id_scratch, &mut ctx.encoding_scratch);
        let encoded_new_layer = &ctx.encoding_scratch;

        // 5. Calculate new size and fill gaps
        let gap_count = if layer > max_level_found {
            (layer - max_level_found) as usize
        } else {
            0
        };

        let new_size = start + gap_count + encoded_new_layer.len() + (valid_end - end);

        // 6. Alloc new space
        let (new_offset, new_capacity) = self.neighbors.alloc(new_size)?;

        let new_offset_u = new_offset as usize;
        let old_offset_u = old_offset as usize;

        // 7. Write parts using copy_within
        // Part 1: Before target
        if start > 0 {
            self.neighbors
                .buffer
                .copy_within(old_offset_u..old_offset_u + start, new_offset_u);
        }

        let mut curr = new_offset_u + start;

        // Part 2: Gaps (empty layers)
        for _ in 0..gap_count {
            self.neighbors.buffer[curr] = 0; // Count = 0 (valid VByte)
            curr += 1;
        }

        // Part 3: New Layer
        self.neighbors.buffer[curr..curr + encoded_new_layer.len()]
            .copy_from_slice(encoded_new_layer);
        curr += encoded_new_layer.len();

        // Part 4: After target
        let remaining = valid_end.saturating_sub(end);
        if remaining > 0 {
            let src_start = old_offset_u + end;
            self.neighbors
                .buffer
                .copy_within(src_start..src_start + remaining, curr);
            curr += remaining;
        }

        // CRITICAL: Zero out the rest
        let allocated_end = new_offset_u + new_capacity as usize;
        if curr < allocated_end {
            self.neighbors.buffer[curr..allocated_end].fill(0);
        }

        // 8. Update node and free old memory
        let node = &mut self.nodes[node_idx];
        if old_len > 0 {
            self.neighbors.free(old_offset, old_len);
        }
        node.neighbor_offset = new_offset;
        node.neighbor_len = new_capacity;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hnsw::config::HnswConfig;
    use crate::storage::VectorStorage;

    #[test]
    fn test_insert_lifecycle() {
        let config = HnswConfig::new(2);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).unwrap();

        let vec1 = vec![1.0, 1.0];
        let vec2 = vec![2.0, 2.0];
        let vec3 = vec![1.0, 2.0];

        let _id1 = index.insert(&vec1, &mut storage).unwrap();
        let _id2 = index.insert(&vec2, &mut storage).unwrap();
        let _id3 = index.insert(&vec3, &mut storage).unwrap();

        assert_eq!(index.node_count(), 3);
        assert!(index.entry_point().is_some());

        // Connectivity check
        let node1 = index.get_node(NodeId(0)).unwrap();
        // We need to read layer 0
        let blob1 = &index.neighbors.buffer[node1.neighbor_offset as usize
            ..node1.neighbor_offset as usize + node1.neighbor_len as usize];
        let neighbors1 = NeighborPool::decode_layer(blob1, 0);

        // Should have neighbors (2 or 3 or both)
        assert!(!neighbors1.is_empty(), "Node 1 should be connected");
    }

    #[test]
    fn test_scan_blob() {
        // [count=2] [1] [2] (layer 0)
        // [count=1] [3] (layer 1)
        // [count=0] (layer 2)
        let l0 = NeighborPool::encode_neighbors(&[1, 2]);
        let l1 = NeighborPool::encode_neighbors(&[3]);
        let l2 = NeighborPool::encode_neighbors(&[]);

        let mut blob = Vec::new();
        blob.extend_from_slice(&l0);
        blob.extend_from_slice(&l1);
        blob.extend_from_slice(&l2);

        let (s0, e0, m0) = scan_blob_for_layer(&blob, 0);
        assert_eq!(s0, 0);
        assert_eq!(e0, l0.len());
        assert_eq!(m0, 0); // Found at level 0

        let (s1, e1, m1) = scan_blob_for_layer(&blob, 1);
        assert_eq!(s1, l0.len());
        assert_eq!(e1, l0.len() + l1.len());
        assert_eq!(m1, 1);

        let (s2, e2, m2) = scan_blob_for_layer(&blob, 2);
        assert_eq!(s2, l0.len() + l1.len());
        assert_eq!(e2, blob.len());
        assert_eq!(m2, 2);

        let (s3, e3, m3) = scan_blob_for_layer(&blob, 3);
        assert_eq!(s3, blob.len());
        assert_eq!(e3, blob.len());
        assert_eq!(m3, 3); // Scanned 3 levels (0, 1, 2) then finished
    }
}

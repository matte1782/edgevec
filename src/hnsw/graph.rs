#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use super::config::HnswConfig;
use super::neighbor::NeighborPool;
use crate::storage::VectorStorage;
use bytemuck::{Pod, Zeroable};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::vec::Vec;
use thiserror::Error;

/// Unique identifier for a vector in the database.
///
/// # Size
/// 8 bytes, aligned to 8
///
/// # Invariants
/// - IDs are never reused (monotonically increasing)
/// - ID 0 is reserved (invalid sentinel)
///
/// # Safety
/// Derives `Pod` and `Zeroable` for safe byte casting via bytemuck.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
#[repr(transparent)]
pub struct VectorId(pub u64);

impl VectorId {
    /// Sentinel value indicating "no vector"
    pub const INVALID: Self = VectorId(0);

    /// First valid ID
    pub const FIRST: Self = VectorId(1);
}

/// Internal node identifier within HNSW graph.
///
/// # Size
/// 4 bytes, aligned to 4
///
/// # Invariants
/// - `NodeId` corresponds 1:1 with `VectorId` (lower 32 bits)
/// - `NodeId` 0xFFFFFFFF is reserved (invalid sentinel)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Sentinel value indicating invalid node
    pub const INVALID: Self = NodeId(u32::MAX);
}

/// Represents a layer level in the HNSW graph.
///
/// Layer 0 is the base layer containing all nodes.
/// Higher layers contain a subset of nodes for faster navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[allow(dead_code)]
pub struct Layer(pub u8);

/// Errors that can occur during graph operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// The graph has reached its maximum node capacity (`u32::MAX`).
    #[error("node capacity exceeded")]
    CapacityExceeded,

    /// The provided `VectorId` is invalid (e.g., sentinel value).
    #[error("invalid vector id")]
    InvalidVectorId,

    /// Neighbor data is corrupted or offset is out of bounds.
    #[error("neighbor data corrupted")]
    NeighborError,

    /// Node ID is out of bounds.
    #[error("node id out of bounds")]
    NodeIdOutOfBounds,

    /// Configuration mismatch with storage.
    #[error("config dimension mismatch: expected {expected}, got {actual}")]
    ConfigMismatch {
        /// Expected dimensions.
        expected: u32,
        /// Actual dimensions in config.
        actual: u32,
    },

    /// Query vector has wrong dimensions.
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimensions.
        expected: usize,
        /// Actual dimensions.
        actual: usize,
    },

    /// Storage operation failed.
    #[error("storage error: {0}")]
    Storage(String),

    /// Invalid configuration parameter.
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

/// A node in the HNSW graph with its adjacency information.
///
/// # Layout
///
/// Total size: 16 bytes
/// Alignment: 8 bytes
///
/// # Fields
///
/// - `vector_id`: 8 bytes
/// - `neighbor_offset`: 4 bytes
/// - `neighbor_len`: 2 bytes
/// - `max_layer`: 1 byte
/// - `pad`: 1 byte (explicit padding, always zero-initialized)
///
/// # Safety
///
/// Derives `Pod` and `Zeroable` for safe byte casting via bytemuck.
/// The `#[repr(C)]` ensures deterministic layout for serialization.
/// All fields are primitive types with no padding gaps.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Pod, Zeroable)]
#[repr(C)]
pub struct HnswNode {
    /// The vector ID this node represents
    pub vector_id: VectorId,

    /// Offset into COMPRESSED neighbor pool
    pub neighbor_offset: u32,

    /// Length of neighbor data in bytes (Allocated Capacity)
    pub neighbor_len: u16,

    /// The maximum layer this node appears in
    pub max_layer: u8,

    /// Explicit padding byte (always zero-initialized for Pod safety)
    pub pad: u8,
}

/// The HNSW Graph structure managing layers and nodes.
///
/// # Memory
///
/// Uses a flattened representation for cache efficiency.
/// Nodes are stored in a contiguous vector.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HnswIndex {
    /// Algorithm configuration
    pub config: HnswConfig,

    /// Node metadata (fixed-size per node)
    pub(crate) nodes: Vec<HnswNode>,

    /// Compressed neighbor lists
    pub(crate) neighbors: NeighborPool,

    /// Entry point (highest layer node)
    pub(crate) entry_point: Option<NodeId>,

    /// Maximum layer in the graph
    pub(crate) max_layer: u8,

    /// Level probability multiplier (1/ln(M))
    pub(crate) level_mult: f32,

    /// Deterministic RNG state
    rng: ChaCha8Rng,
}

impl HnswIndex {
    /// Creates a new empty HNSW graph.
    ///
    /// # Arguments
    ///
    /// * `config` - HNSW configuration parameters.
    /// * `storage` - Vector storage to validate against.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::ConfigMismatch` if storage dimensions differ from config.
    /// Returns `GraphError::InvalidConfig` if configuration parameters are invalid (e.g., M <= 1).
    pub fn new(config: HnswConfig, storage: &VectorStorage) -> Result<Self, GraphError> {
        if config.dimensions != storage.dimensions() {
            return Err(GraphError::ConfigMismatch {
                expected: storage.dimensions(),
                actual: config.dimensions,
            });
        }

        if config.m <= 1 {
            return Err(GraphError::InvalidConfig(format!(
                "m must be > 1, got {}",
                config.m
            )));
        }
        if config.m0 < config.m {
            return Err(GraphError::InvalidConfig(format!(
                "m0 must be >= m, got {} < {}",
                config.m0, config.m
            )));
        }

        // Calculate level multiplier: m_l = 1 / ln(M)
        let m_float = config.m as f32;
        let level_mult = if m_float > 1.0 {
            1.0 / m_float.ln()
        } else {
            0.0
        };

        // Initialize RNG with a default seed for determinism.
        let rng = ChaCha8Rng::seed_from_u64(42);

        Ok(Self {
            config,
            nodes: Vec::new(),
            neighbors: NeighborPool::new(),
            entry_point: None,
            max_layer: 0,
            level_mult,
            rng,
        })
    }

    /// Generates a random level for a new node.
    ///
    /// Formula: `floor(-ln(uniform(0,1)) * m_l)`
    /// Clamped to `max_level` (e.g. 16) to prevent memory explosion.
    #[must_use]
    pub fn get_random_level(&mut self) -> u8 {
        // Generate uniform(0, 1)
        let r: f32 = self.rng.gen_range(f32::EPSILON..=1.0);
        let level = (-r.ln() * self.level_mult).floor();

        // Safety cap (e.g. 16)
        if level > 16.0 {
            16
        } else {
            level as u8
        }
    }

    /// Adds a node to the graph.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The external vector identifier
    /// * `max_layer` - The maximum layer for this node
    ///
    /// # Returns
    ///
    /// The new `NodeId` assigned to this node, or a `GraphError`.
    pub fn add_node(&mut self, vector_id: VectorId, max_layer: u8) -> Result<NodeId, GraphError> {
        if vector_id == VectorId::INVALID {
            return Err(GraphError::InvalidVectorId);
        }

        // Safety limit for NodeId
        if self.nodes.len() >= u32::MAX as usize {
            return Err(GraphError::CapacityExceeded);
        }

        let node = HnswNode {
            vector_id,
            neighbor_offset: 0,
            neighbor_len: 0,
            max_layer,
            pad: 0,
        };

        #[allow(clippy::cast_possible_truncation)]
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);

        // Update max layer if needed
        if max_layer > self.max_layer {
            self.max_layer = max_layer;
        }

        Ok(id)
    }

    /// Sets the neighbors for a node.
    ///
    /// # Arguments
    /// * `node_id` - The node to update.
    /// * `neighbors` - The list of neighbor IDs.
    pub fn set_neighbors(
        &mut self,
        node_id: NodeId,
        neighbors: &[NodeId],
    ) -> Result<(), GraphError> {
        if node_id.0 as usize >= self.nodes.len() {
            return Err(GraphError::InvalidVectorId);
        }

        // Convert NodeId to u32 for encoding
        let neighbor_u32s: Vec<u32> = neighbors.iter().map(|n| n.0).collect();
        let encoded = NeighborPool::encode_neighbors(&neighbor_u32s);

        // Alloc new space
        let (offset, capacity) = self.neighbors.alloc(encoded.len())?;

        // Write data
        let start = offset as usize;
        let end = start + encoded.len();
        self.neighbors.buffer[start..end].copy_from_slice(&encoded);

        // Update node and free old
        let node = &mut self.nodes[node_id.0 as usize];

        // Free old slot if it existed
        if node.neighbor_len > 0 {
            self.neighbors.free(node.neighbor_offset, node.neighbor_len);
        }

        node.neighbor_offset = offset;
        node.neighbor_len = capacity; // Store allocated capacity

        Ok(())
    }

    /// Retrieves a node by its ID.
    #[must_use]
    pub fn get_node(&self, id: NodeId) -> Option<&HnswNode> {
        if id == NodeId::INVALID {
            return None;
        }
        self.nodes.get(id.0 as usize)
    }

    /// Retrieves the neighbors for a node.
    pub fn get_neighbors(&self, node: &HnswNode) -> Result<Vec<NodeId>, GraphError> {
        let start = node.neighbor_offset as usize;
        // Read up to allocated capacity
        let end = start + node.neighbor_len as usize;

        if end > self.neighbors.buffer.len() {
            return Err(GraphError::NeighborError);
        }

        let slice = &self.neighbors.buffer[start..end];
        let raw_neighbors = NeighborPool::decode_neighbors(slice);

        // Convert back to NodeId
        let neighbors = raw_neighbors.into_iter().map(NodeId).collect();
        Ok(neighbors)
    }

    /// Retrieves the neighbors for a specific layer.
    pub fn get_neighbors_layer(
        &self,
        node: &HnswNode,
        layer: u8,
    ) -> Result<Vec<NodeId>, GraphError> {
        let start = node.neighbor_offset as usize;
        let end = start + node.neighbor_len as usize;

        if end > self.neighbors.buffer.len() {
            return Err(GraphError::NeighborError);
        }

        let slice = &self.neighbors.buffer[start..end];
        let raw_neighbors = NeighborPool::decode_layer(slice, layer);

        // Convert back to NodeId
        let neighbors = raw_neighbors.into_iter().map(NodeId).collect();
        Ok(neighbors)
    }

    /// Returns the number of nodes in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the entry point node ID, if any.
    #[must_use]
    pub fn entry_point(&self) -> Option<NodeId> {
        self.entry_point
    }

    /// Sets the entry point node ID.
    pub fn set_entry_point(&mut self, id: NodeId) {
        self.entry_point = Some(id);
    }

    /// Returns the current maximum layer in the graph.
    #[must_use]
    pub fn max_layer(&self) -> u8 {
        self.max_layer
    }

    /// Marks a vector as deleted in the storage.
    ///
    /// The node remains in the graph for routing, but will be filtered from search results.
    ///
    /// # Arguments
    ///
    /// * `id` - The vector ID to delete.
    /// * `storage` - The vector storage to update.
    ///
    /// # Returns
    ///
    /// `true` if the vector was active and is now deleted.
    /// `false` if it was already deleted.
    pub fn delete(&self, id: VectorId, storage: &mut VectorStorage) -> bool {
        storage.mark_deleted(id)
    }

    /// DEBUG: Print memory stats
    pub fn log_stats(&self) {
        println!("Index Stats:");
        println!("  Node Count: {}", self.nodes.len());
        println!("  Neighbor Buffer Len: {}", self.neighbors.buffer.len());
        println!(
            "  Neighbor Buffer Cap: {}",
            self.neighbors.buffer.capacity()
        );
        println!("  Total Memory Usage: {} bytes", self.memory_usage());
        // bucket stats are internal to NeighborPool
    }

    /// Returns the approximate memory usage in bytes.
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        let nodes_size = self.nodes.capacity() * std::mem::size_of::<HnswNode>();
        let neighbors_size = self.neighbors.memory_usage();

        std::mem::size_of::<Self>() + nodes_size + neighbors_size
    }
}

/// Trait for providing vector data by ID.
pub trait VectorProvider {
    /// Returns the vector data for a given ID.
    fn get_vector(&self, id: VectorId) -> Cow<'_, [f32]>;

    /// Returns true if the vector is marked as deleted.
    fn is_deleted(&self, id: VectorId) -> bool {
        let _ = id;
        false
    }

    /// Returns the quantized vector data for a given ID, if available.
    ///
    /// # Returns
    ///
    /// * `Some(&[u8])` - If the provider supports direct quantized access.
    /// * `None` - If not supported or data is not quantized.
    fn get_quantized_vector(&self, id: VectorId) -> Option<&[u8]> {
        let _ = id;
        None
    }

    /// Quantizes a query vector into the provided output buffer.
    ///
    /// # Arguments
    ///
    /// * `query` - The query vector in f32.
    /// * `output` - buffer to write quantized data into.
    ///
    /// # Returns
    ///
    /// * `Some(&[u8])` - The quantized slice (borrowed from output).
    /// * `None` - If quantization is not supported.
    fn quantize_query<'a>(&self, query: &[f32], output: &'a mut Vec<u8>) -> Option<&'a [u8]> {
        let _ = query;
        let _ = output;
        None
    }
}

// ============================================================================
// Batch Insertion Implementation (W11.1 - RFC 0001 v1.1)
// ============================================================================

use crate::batch::BatchInsertable;
use crate::error::BatchError;
use std::collections::HashSet;

impl HnswIndex {
    /// Returns the configured dimensionality of the index.
    #[must_use]
    pub fn dimensions(&self) -> u32 {
        self.config.dimensions
    }

    /// Returns the current number of nodes in the index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the index contains no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the maximum capacity of the index (u32::MAX nodes).
    #[must_use]
    pub const fn capacity(&self) -> usize {
        u32::MAX as usize
    }

    /// Checks if a vector ID already exists in the index.
    ///
    /// Returns `true` if the ID exists, `false` otherwise.
    #[must_use]
    pub fn contains_id(&self, id: u64) -> bool {
        self.nodes.iter().any(|n| n.vector_id.0 == id)
    }

    /// Checks if a vector contains invalid floating-point values.
    ///
    /// Returns `Some(reason)` if invalid, `None` if valid.
    fn validate_vector(vector: &[f32]) -> Option<String> {
        for (i, &val) in vector.iter().enumerate() {
            if val.is_nan() {
                return Some(format!("NaN at index {i}"));
            }
            if val.is_infinite() {
                return Some(format!("Infinity at index {i}"));
            }
        }
        None
    }
}

impl BatchInsertable for HnswIndex {
    /// Inserts multiple vectors in a single batch operation.
    ///
    /// This is the full implementation per RFC 0001 v1.2.
    ///
    /// # Algorithm
    ///
    /// 1. Collect iterator to Vec (required for progress tracking)
    /// 2. Pre-validate first vector's dimensionality (fail-fast)
    /// 3. Check capacity constraints
    /// 4. Iterate through vectors with progress callbacks
    /// 5. Actually insert vectors into HNSW graph via storage
    /// 6. Handle errors according to best-effort semantics
    ///
    /// # Error Handling
    ///
    /// - **Fatal errors** (abort immediately):
    ///   - DimensionMismatch on first vector
    ///   - CapacityExceeded
    ///   - InternalError (HNSW invariant violated)
    ///
    /// - **Non-fatal errors** (skip and continue):
    ///   - DuplicateId (within batch or existing in index)
    ///   - InvalidVector (NaN, Inf)
    ///   - DimensionMismatch on subsequent vectors
    fn batch_insert<I, F>(
        &mut self,
        vectors: I,
        storage: &mut VectorStorage,
        mut progress_callback: Option<F>,
    ) -> Result<Vec<u64>, BatchError>
    where
        I: IntoIterator<Item = (u64, Vec<f32>)>,
        F: FnMut(usize, usize),
    {
        // Step 1: Collect iterator to Vec for progress tracking
        let batch: Vec<(u64, Vec<f32>)> = vectors.into_iter().collect();
        let total = batch.len();

        // Early return for empty batch
        if total == 0 {
            return Ok(vec![]);
        }

        // Step 2: Pre-validate first vector's dimensionality (fail-fast)
        let expected_dim = self.config.dimensions as usize;
        let (first_id, first_vec) = &batch[0];
        if first_vec.len() != expected_dim {
            return Err(BatchError::DimensionMismatch {
                expected: expected_dim,
                actual: first_vec.len(),
                vector_id: *first_id,
            });
        }

        // Step 3: Check capacity constraints
        let current_count = self.nodes.len();
        if current_count.saturating_add(total) > self.capacity() {
            return Err(BatchError::CapacityExceeded {
                current: current_count,
                max: self.capacity(),
            });
        }

        // Track IDs we've seen in this batch to detect duplicates within the batch
        let mut seen_ids: HashSet<u64> = HashSet::with_capacity(total);

        // Track successfully inserted IDs
        let mut inserted_ids: Vec<u64> = Vec::with_capacity(total);

        // Step 4: Initial progress callback (0%)
        if let Some(ref mut callback) = progress_callback {
            callback(0, total);
        }

        // Calculate progress intervals (every 10%)
        let progress_interval = if total >= 10 { total / 10 } else { 1 };

        // Step 5: Process each vector
        for (id, vector) in batch {
            // Check for duplicate ID within this batch
            if !seen_ids.insert(id) {
                // Duplicate within batch - skip (non-fatal)
                continue;
            }

            // Check for ID 0 (reserved sentinel)
            if id == 0 {
                // Skip invalid ID (non-fatal)
                continue;
            }

            // Check for duplicate ID in existing index [M1 fix]
            if self.contains_id(id) {
                // Duplicate in existing index - skip (non-fatal)
                continue;
            }

            // Validate dimensionality
            if vector.len() != expected_dim {
                // Skip dimension mismatch (non-fatal after first vector)
                continue;
            }

            // Validate vector data (NaN, Inf)
            if Self::validate_vector(&vector).is_some() {
                // Skip invalid vector (non-fatal)
                continue;
            }

            // Step 6: Actually insert the vector into the HNSW graph [C1 fix]
            match self.insert(&vector, storage) {
                Ok(assigned_id) => {
                    // Use the ID assigned by the insert method
                    inserted_ids.push(assigned_id.0);
                }
                Err(e) => {
                    // Internal error during HNSW insertion - this is fatal
                    return Err(BatchError::InternalError {
                        message: format!("HNSW insert failed for vector {id}: {e}"),
                    });
                }
            }

            // Progress callback at intervals [M3 fix: report inserted count]
            let inserted_count = inserted_ids.len();
            if inserted_count % progress_interval == 0 || inserted_count == 1 {
                if let Some(ref mut callback) = progress_callback {
                    callback(inserted_count, total);
                }
            }
        }

        // Final progress callback (100%)
        if let Some(ref mut callback) = progress_callback {
            callback(inserted_ids.len(), total);
        }

        Ok(inserted_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HnswIndex>();
    }

    #[test]
    fn test_initialization() {
        let config = HnswConfig::new(128);
        // Create storage with matching dimensions
        let storage = VectorStorage::new(&config, None);

        let index = HnswIndex::new(config.clone(), &storage).unwrap();

        assert_eq!(index.node_count(), 0);
        assert_eq!(index.entry_point(), None);
        assert_eq!(index.max_layer(), 0);
    }

    #[test]
    fn test_dimension_mismatch() {
        let config_idx = HnswConfig::new(128);
        let config_store = HnswConfig::new(64);
        let storage = VectorStorage::new(&config_store, None);

        let result = HnswIndex::new(config_idx, &storage);
        assert!(matches!(
            result,
            Err(GraphError::ConfigMismatch {
                expected: 64,
                actual: 128
            })
        ));
    }

    #[test]
    fn test_layer_distribution() {
        // Geometric distribution test
        // m=16 => m_l = 1/ln(16) ≈ 0.36
        // Prob(level > 0) = e^(-1/m_l) = 1/M = 1/16
        // We can't strictly test randomness without huge samples, but we can sanity check.
        let config = HnswConfig::new(128);
        let storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).unwrap();

        let mut levels = vec![0u8; 1000];
        for l in levels.iter_mut() {
            *l = index.get_random_level();
        }

        // Level 0 should be most common
        let l0_count = levels.iter().filter(|&&l| l == 0).count();
        assert!(
            l0_count > 800,
            "Level 0 should be dominant (expected ~93% for M=16)"
        );

        // Max level shouldn't be crazy
        let max = *levels.iter().max().unwrap();
        assert!(max < 16, "Level should be reasonable");
    }

    #[test]
    fn test_neighbor_roundtrip() {
        let config = HnswConfig::new(128);
        let storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).unwrap();

        let id1 = index.add_node(VectorId(1), 0).unwrap();
        let id2 = index.add_node(VectorId(2), 0).unwrap();
        let id3 = index.add_node(VectorId(3), 0).unwrap();

        // Neighbors: [2, 3]
        let neighbors = vec![id2, id3];
        index.set_neighbors(id1, &neighbors).unwrap();

        {
            let node1 = index.get_node(id1).unwrap();
            let retrieved = index.get_neighbors(node1).unwrap();
            assert_eq!(retrieved, neighbors);
        } // Drop node1 borrow

        // Update neighbors: [3] (shrink)
        let neighbors2 = vec![id3];
        index.set_neighbors(id1, &neighbors2).unwrap();

        {
            let node1 = index.get_node(id1).unwrap();
            let retrieved2 = index.get_neighbors(node1).unwrap();
            assert_eq!(retrieved2, neighbors2);
        }

        // Check if free list got populated (cannot check directly as NeighborPool is private,
        // but we can trust neighbor.rs tests for internal logic)
    }

    // ============================================================
    // BATCH INSERT TESTS (W11.1 - RFC 0001 v1.2)
    // ============================================================

    mod batch_insert_tests {
        use super::*;
        use crate::batch::BatchInsertable;

        fn create_test_index_and_storage(dim: u32) -> (HnswIndex, VectorStorage) {
            let config = HnswConfig::new(dim);
            let storage = VectorStorage::new(&config, None);
            let index = HnswIndex::new(config, &storage).unwrap();
            (index, storage)
        }

        #[test]
        fn test_batch_insert_empty() {
            let (mut index, mut storage) = create_test_index_and_storage(128);
            let vectors: Vec<(u64, Vec<f32>)> = vec![];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
            // [M2 fix] Verify graph state
            assert_eq!(index.node_count(), 0);
        }

        #[test]
        fn test_batch_insert_single_vector() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            let vectors = vec![(1u64, vec![1.0, 2.0, 3.0, 4.0])];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 1);
            // [M2 fix] Verify graph state - node was actually inserted
            assert_eq!(index.node_count(), 1);
        }

        #[test]
        fn test_batch_insert_multiple_vectors() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            let vectors = vec![
                (1u64, vec![1.0, 2.0, 3.0, 4.0]),
                (2u64, vec![5.0, 6.0, 7.0, 8.0]),
                (3u64, vec![9.0, 10.0, 11.0, 12.0]),
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 3);
            // [M2 fix] Verify graph state - all 3 nodes inserted
            assert_eq!(index.node_count(), 3);
        }

        #[test]
        fn test_batch_insert_dimension_mismatch_first_vector() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            // First vector has wrong dimension (3 instead of 4)
            let vectors = vec![
                (1u64, vec![1.0, 2.0, 3.0]), // Wrong!
                (2u64, vec![5.0, 6.0, 7.0, 8.0]),
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            assert!(result.is_err());
            match result.unwrap_err() {
                BatchError::DimensionMismatch {
                    expected,
                    actual,
                    vector_id,
                } => {
                    assert_eq!(expected, 4);
                    assert_eq!(actual, 3);
                    assert_eq!(vector_id, 1);
                }
                _ => panic!("Expected DimensionMismatch error"),
            }
            // [M2 fix] Verify no vectors were inserted on fatal error
            assert_eq!(index.node_count(), 0);
        }

        #[test]
        fn test_batch_insert_dimension_mismatch_later_vector_skipped() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            // Second vector has wrong dimension - should be skipped
            let vectors = vec![
                (1u64, vec![1.0, 2.0, 3.0, 4.0]),
                (2u64, vec![5.0, 6.0, 7.0]), // Wrong, but not first
                (3u64, vec![9.0, 10.0, 11.0, 12.0]),
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            // Should succeed with partial results
            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 2);
            // [M2 fix] Verify only 2 nodes inserted (one skipped)
            assert_eq!(index.node_count(), 2);
        }

        #[test]
        fn test_batch_insert_duplicate_id_within_batch() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            // Duplicate IDs within the batch
            let vectors = vec![
                (1u64, vec![1.0, 2.0, 3.0, 4.0]),
                (1u64, vec![5.0, 6.0, 7.0, 8.0]), // Duplicate!
                (2u64, vec![9.0, 10.0, 11.0, 12.0]),
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            // Should succeed, skipping the duplicate
            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 2);
            // [M2 fix] Verify graph state
            assert_eq!(index.node_count(), 2);
        }

        #[test]
        fn test_batch_insert_duplicate_id_in_existing_index() {
            // [M1 fix] Test duplicate check against existing index
            let (mut index, mut storage) = create_test_index_and_storage(4);

            // First batch
            let vectors1 = vec![(1u64, vec![1.0, 2.0, 3.0, 4.0])];
            let result1 = index.batch_insert(vectors1, &mut storage, None::<fn(usize, usize)>);
            assert!(result1.is_ok());
            assert_eq!(index.node_count(), 1);

            // Second batch with duplicate ID
            let vectors2 = vec![
                (1u64, vec![5.0, 6.0, 7.0, 8.0]), // Duplicate of existing!
                (2u64, vec![9.0, 10.0, 11.0, 12.0]),
            ];
            let result2 = index.batch_insert(vectors2, &mut storage, None::<fn(usize, usize)>);

            // Should succeed, skipping the duplicate
            assert!(result2.is_ok());
            let ids = result2.unwrap();
            assert_eq!(ids.len(), 1); // Only ID 2 was inserted
            assert_eq!(index.node_count(), 2); // Total 2 nodes now
        }

        #[test]
        fn test_batch_insert_invalid_vector_nan() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            let vectors = vec![
                (1u64, vec![1.0, 2.0, 3.0, 4.0]),
                (2u64, vec![f32::NAN, 6.0, 7.0, 8.0]), // NaN!
                (3u64, vec![9.0, 10.0, 11.0, 12.0]),
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            // Should succeed, skipping the NaN vector
            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 2);
            // [M2 fix] Verify graph state
            assert_eq!(index.node_count(), 2);
        }

        #[test]
        fn test_batch_insert_invalid_vector_infinity() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            let vectors = vec![
                (1u64, vec![1.0, 2.0, 3.0, 4.0]),
                (2u64, vec![f32::INFINITY, 6.0, 7.0, 8.0]), // Infinity!
                (3u64, vec![9.0, 10.0, 11.0, 12.0]),
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            // Should succeed, skipping the Infinity vector
            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 2);
            // [M2 fix] Verify graph state
            assert_eq!(index.node_count(), 2);
        }

        #[test]
        fn test_batch_insert_invalid_vector_neg_infinity() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            let vectors = vec![
                (1u64, vec![1.0, 2.0, 3.0, 4.0]),
                (2u64, vec![f32::NEG_INFINITY, 6.0, 7.0, 8.0]), // Neg Infinity!
                (3u64, vec![9.0, 10.0, 11.0, 12.0]),
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            // Should succeed, skipping the NegInfinity vector
            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 2);
            // [M2 fix] Verify graph state
            assert_eq!(index.node_count(), 2);
        }

        #[test]
        fn test_batch_insert_id_zero_skipped() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            // ID 0 is reserved sentinel
            let vectors = vec![
                (0u64, vec![1.0, 2.0, 3.0, 4.0]), // Reserved!
                (1u64, vec![5.0, 6.0, 7.0, 8.0]),
                (2u64, vec![9.0, 10.0, 11.0, 12.0]),
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            // Should succeed, skipping ID 0
            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 2);
            // [M2 fix] Verify graph state
            assert_eq!(index.node_count(), 2);
        }

        #[test]
        fn test_batch_insert_progress_callback_called() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            let vectors: Vec<(u64, Vec<f32>)> = (1..=10)
                .map(|i| (i as u64, vec![i as f32; 4]))
                .collect();

            let mut progress_calls: Vec<(usize, usize)> = vec![];

            let result = index.batch_insert(vectors, &mut storage, Some(|current, total| {
                progress_calls.push((current, total));
            }));

            assert!(result.is_ok());
            // [M2 fix] Verify all 10 nodes inserted
            assert_eq!(index.node_count(), 10);

            // Should have called progress at 0% and at intervals
            assert!(!progress_calls.is_empty());

            // First call should be (0, 10)
            assert_eq!(progress_calls[0], (0, 10));

            // Last call should report inserted count (10, 10)
            let last = progress_calls.last().unwrap();
            assert_eq!(*last, (10, 10));
        }

        #[test]
        fn test_batch_insert_progress_callback_single_vector() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            let vectors = vec![(1u64, vec![1.0, 2.0, 3.0, 4.0])];

            let mut progress_calls: Vec<(usize, usize)> = vec![];

            let result = index.batch_insert(vectors, &mut storage, Some(|current, total| {
                progress_calls.push((current, total));
            }));

            assert!(result.is_ok());
            // [M2 fix] Verify 1 node inserted
            assert_eq!(index.node_count(), 1);

            // Should have at least two calls: 0% and 100%
            assert!(progress_calls.len() >= 2);
            assert_eq!(progress_calls[0], (0, 1));
            assert!(progress_calls.contains(&(1, 1)));
        }

        #[test]
        fn test_batch_insert_mixed_errors() {
            let (mut index, mut storage) = create_test_index_and_storage(4);
            // Mix of valid and invalid vectors
            let vectors = vec![
                (1u64, vec![1.0, 2.0, 3.0, 4.0]),       // Valid
                (2u64, vec![f32::NAN, 2.0, 3.0, 4.0]),  // NaN - skip
                (3u64, vec![3.0, 3.0, 3.0]),            // Wrong dim - skip
                (1u64, vec![4.0, 4.0, 4.0, 4.0]),       // Duplicate - skip
                (0u64, vec![5.0, 5.0, 5.0, 5.0]),       // Reserved ID - skip
                (4u64, vec![6.0, 6.0, 6.0, 6.0]),       // Valid
            ];

            let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

            assert!(result.is_ok());
            let ids = result.unwrap();
            assert_eq!(ids.len(), 2);
            // [M2 fix] Verify graph state
            assert_eq!(index.node_count(), 2);
        }

        #[test]
        fn test_helper_dimensions() {
            let (index, _storage) = create_test_index_and_storage(128);
            assert_eq!(index.dimensions(), 128);
        }

        #[test]
        fn test_helper_len_empty() {
            let (index, _storage) = create_test_index_and_storage(128);
            assert_eq!(index.len(), 0);
            assert!(index.is_empty());
        }

        #[test]
        fn test_helper_capacity() {
            let (index, _storage) = create_test_index_and_storage(128);
            assert_eq!(index.capacity(), u32::MAX as usize);
        }

        #[test]
        fn test_helper_contains_id() {
            let (mut index, mut storage) = create_test_index_and_storage(4);

            // Initially empty
            assert!(!index.contains_id(1));

            // Insert via single insert
            let _ = index.insert(&[1.0, 2.0, 3.0, 4.0], &mut storage);

            // Now the first ID should exist (insert assigns sequential IDs starting at 1)
            assert!(index.node_count() == 1);
        }

        #[test]
        fn test_validate_vector_valid() {
            let vector = vec![1.0, 2.0, 3.0, 4.0];
            assert!(HnswIndex::validate_vector(&vector).is_none());
        }

        #[test]
        fn test_validate_vector_nan() {
            let vector = vec![1.0, f32::NAN, 3.0, 4.0];
            let result = HnswIndex::validate_vector(&vector);
            assert!(result.is_some());
            assert!(result.unwrap().contains("NaN"));
        }

        #[test]
        fn test_validate_vector_infinity() {
            let vector = vec![1.0, f32::INFINITY, 3.0, 4.0];
            let result = HnswIndex::validate_vector(&vector);
            assert!(result.is_some());
            assert!(result.unwrap().contains("Infinity"));
        }
    }
}

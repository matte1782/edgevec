//! HNSW graph implementation.
//!
//! # Lint Suppressions
//!
//! This module suppresses several numeric cast lints at the module level because:
//!
//! - **cast_possible_truncation**: The HNSW algorithm uses `u32` for node IDs and neighbor
//!   indices. While `usize` is used for array indexing, all values are validated to fit in
//!   `u32` at insertion time. Maximum supported index size is 4 billion vectors.
//!
//! - **cast_precision_loss**: Float calculations for level assignment use `f64` internally
//!   and convert to `usize` for layer counts. Precision loss is acceptable as layers are
//!   discrete integers (typically 0-16).
//!
//! - **cast_sign_loss**: Some internal calculations use signed intermediates that are
//!   guaranteed non-negative by algorithm invariants.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use super::config::HnswConfig;
use super::neighbor::NeighborPool;
use crate::metadata::{MetadataError, MetadataStore, MetadataValue};
use crate::quantization::variable::BinaryVector;
use crate::storage::binary::BinaryVectorStorage;
use crate::storage::VectorStorage;
use bytemuck::{Pod, Zeroable};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
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

    /// Metadata validation failed (v0.6.0 - RFC-002).
    ///
    /// Returned when metadata fails validation during `insert_with_metadata()`.
    /// The index remains unchanged when this error occurs.
    #[error("metadata validation failed: {0}")]
    MetadataValidation(#[from] MetadataError),

    /// Filter parsing failed (v0.6.0 - RFC-002).
    ///
    /// Returned when a filter expression string cannot be parsed.
    #[error("filter parse error: {0}")]
    FilterParse(String),

    /// Filter evaluation failed (v0.6.0 - RFC-002).
    ///
    /// Returned when a filter expression evaluation fails at runtime.
    #[error("filter evaluation error: {0}")]
    FilterEval(String),

    /// Binary quantization is not enabled (v0.7.0 - RFC-002 Phase 2).
    ///
    /// Returned when attempting BQ operations on an index without BQ storage.
    #[error("binary quantization is not enabled; use with_bq() or enable_bq() first")]
    BqNotEnabled,

    /// Binary quantization failed (v0.7.0 - RFC-002 Phase 2).
    ///
    /// Returned when vector quantization fails during BQ operations.
    #[error("quantization error: {0}")]
    Quantization(String),
}

/// Result of a compaction operation.
///
/// Returned by [`HnswIndex::compact()`] to provide metrics about the operation.
///
/// # Fields
///
/// * `tombstones_removed` - Number of deleted vectors removed
/// * `new_size` - Size of the new index (live vectors only)
/// * `duration_ms` - Time taken for the operation in milliseconds
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionResult {
    /// Number of tombstones (deleted vectors) removed during compaction.
    pub tombstones_removed: usize,
    /// New index size after compaction (live vectors only).
    pub new_size: usize,
    /// Time taken for the compaction operation in milliseconds.
    pub duration_ms: u64,
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
/// - `deleted`: 1 byte (soft delete flag, v0.3.0)
///
/// # Soft Delete (v0.3.0)
///
/// The `deleted` field enables O(1) soft delete. Deleted nodes remain in
/// the graph for routing but are excluded from search results.
/// - `deleted = 0`: Node is live (default)
/// - `deleted = 1`: Node is deleted (tombstone)
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

    /// Soft delete flag: 0 = live, 1 = deleted (v0.3.0)
    /// This field replaces the padding byte from v0.2.x.
    pub deleted: u8,
}

/// The HNSW Graph structure managing layers and nodes.
///
/// # Memory
///
/// Uses a flattened representation for cache efficiency.
/// Nodes are stored in a contiguous vector.
///
/// # Soft Delete (v0.3.0)
///
/// Supports soft delete via tombstone marking. Deleted nodes remain
/// in the graph for routing but are excluded from search results.
/// Use `compact()` to reclaim space from deleted nodes.
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

    /// Count of soft-deleted vectors (v0.3.0)
    /// This tracks the number of nodes with `deleted = 1`.
    #[serde(default)]
    pub(crate) deleted_count: usize,

    /// Compaction threshold ratio (v0.3.0)
    /// When tombstone_ratio() exceeds this value, needs_compaction() returns true.
    /// Default: 0.3 (30% tombstones triggers compaction recommendation)
    #[serde(default = "default_compaction_threshold")]
    compaction_threshold: f64,

    /// Integrated metadata storage (v0.6.0 - RFC-002)
    ///
    /// Stores key-value metadata attached to vectors. Thread-safe when all
    /// contained types are `Send + Sync` (which they are for String/MetadataValue).
    ///
    /// Concurrent modification requires external synchronization, matching
    /// the existing pattern for other HnswIndex operations.
    #[serde(default)]
    pub(crate) metadata: MetadataStore,

    /// Binary quantization storage (v0.7.0 - RFC-002 Phase 2)
    ///
    /// Optional storage for binary quantized vectors. When enabled, vectors
    /// are stored in both F32 and binary format. BQ provides 32x memory
    /// reduction and 3-5x search speedup at the cost of some recall.
    ///
    /// Use `with_bq()` to create an index with BQ enabled, or `enable_bq()`
    /// to add BQ support to an existing index.
    #[serde(skip)]
    pub(crate) bq_storage: Option<BinaryVectorStorage>,
}

/// Default compaction threshold (30%)
fn default_compaction_threshold() -> f64 {
    0.3
}

/// Error type for individual batch delete failures
/// [C4 FIX] Enables caller to distinguish failure reasons
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchDeleteError {
    /// Vector ID not found in index
    NotFound(VectorId),
    /// Vector was already deleted (idempotent, not an error)
    AlreadyDeleted(VectorId),
    /// Internal error during deletion
    InternalError(VectorId, String),
}

/// Result of a batch delete operation
/// [C4 FIX] Includes detailed error information
#[derive(Debug, Clone)]
pub struct BatchDeleteResult {
    /// Number of vectors successfully deleted
    pub deleted: usize,
    /// Number of vectors that were already deleted (idempotent)
    pub already_deleted: usize,
    /// Number of invalid IDs (not found in index)
    pub invalid_ids: usize,
    /// Total IDs in input (including duplicates)
    pub total: usize,
    /// [m2 FIX] Number of unique IDs processed (duplicates removed)
    pub unique_count: usize,
    /// [C4 FIX] Detailed errors for failed IDs
    pub errors: Vec<BatchDeleteError>,
}

impl BatchDeleteResult {
    /// Create a new empty result
    #[must_use]
    pub fn new() -> Self {
        Self {
            deleted: 0,
            already_deleted: 0,
            invalid_ids: 0,
            total: 0,
            unique_count: 0,
            errors: Vec::new(),
        }
    }

    /// Check if all operations succeeded (no invalid IDs)
    #[must_use]
    pub fn all_valid(&self) -> bool {
        self.invalid_ids == 0
    }

    /// Check if any deletions occurred
    #[must_use]
    pub fn any_deleted(&self) -> bool {
        self.deleted > 0
    }

    /// Check if there were any errors (not including already-deleted)
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.invalid_ids > 0
            || !self
                .errors
                .iter()
                .all(|e| matches!(e, BatchDeleteError::AlreadyDeleted(_)))
    }
}

impl Default for BatchDeleteResult {
    fn default() -> Self {
        Self::new()
    }
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
            deleted_count: 0, // v0.3.0: No deleted nodes initially
            compaction_threshold: default_compaction_threshold(), // v0.3.0: Default 30%
            metadata: MetadataStore::new(), // v0.6.0 RFC-002: Empty metadata store
            bq_storage: None, // v0.7.0 RFC-002 Phase 2: BQ disabled by default
        })
    }

    /// Creates a new HNSW graph with pre-populated metadata (v0.6.0 - RFC-002).
    ///
    /// This constructor allows initializing the index with existing metadata,
    /// useful for deserialization or migration scenarios.
    ///
    /// # Arguments
    ///
    /// * `config` - HNSW configuration parameters.
    /// * `storage` - Vector storage to validate against.
    /// * `metadata` - Pre-populated metadata store.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::ConfigMismatch` if storage dimensions differ from config.
    /// Returns `GraphError::InvalidConfig` if configuration parameters are invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(4);
    /// let storage = VectorStorage::new(&config, None);
    ///
    /// let mut metadata = MetadataStore::new();
    /// metadata.insert(1, "key", MetadataValue::String("value".into())).unwrap();
    ///
    /// let index = HnswIndex::with_metadata(config, &storage, metadata).unwrap();
    /// assert!(index.metadata().has_key(1, "key"));
    /// ```
    pub fn with_metadata(
        config: HnswConfig,
        storage: &VectorStorage,
        metadata: MetadataStore,
    ) -> Result<Self, GraphError> {
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

        let m_float = config.m as f32;
        let level_mult = if m_float > 1.0 {
            1.0 / m_float.ln()
        } else {
            0.0
        };

        let rng = ChaCha8Rng::seed_from_u64(42);

        Ok(Self {
            config,
            nodes: Vec::new(),
            neighbors: NeighborPool::new(),
            entry_point: None,
            max_layer: 0,
            level_mult,
            rng,
            deleted_count: 0,
            compaction_threshold: default_compaction_threshold(),
            metadata, // Use provided metadata
            bq_storage: None,
        })
    }

    /// Creates a new index with binary quantization enabled (v0.7.0 - RFC-002 Phase 2).
    ///
    /// BQ provides 32x memory reduction and 3-5x search speedup at the cost of
    /// some recall (which can be recovered via rescoring).
    ///
    /// # Arguments
    ///
    /// * `config` - HNSW configuration parameters.
    /// * `storage` - Vector storage to validate against.
    ///
    /// # Errors
    ///
    /// Returns error if dimension is not divisible by 8 (required for binary packing).
    /// Returns `GraphError::ConfigMismatch` if storage dimensions differ from config.
    /// Returns `GraphError::InvalidConfig` if configuration parameters are invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(128); // 128D, divisible by 8
    /// let storage = VectorStorage::new(&config, None);
    /// let index = HnswIndex::with_bq(config, &storage).unwrap();
    ///
    /// assert!(index.has_bq());
    /// ```
    pub fn with_bq(config: HnswConfig, storage: &VectorStorage) -> Result<Self, GraphError> {
        let dimension = config.dimensions as usize;

        // Validate dimension is compatible with BQ (divisible by 8)
        if dimension % 8 != 0 {
            return Err(GraphError::InvalidConfig(format!(
                "dimension must be divisible by 8 for binary quantization, got {dimension}"
            )));
        }

        let mut index = Self::new(config, storage)?;

        index.bq_storage = Some(
            BinaryVectorStorage::new(dimension).map_err(|e| GraphError::Storage(e.to_string()))?,
        );

        Ok(index)
    }

    /// Enables binary quantization on an existing index (v0.7.0 - RFC-002 Phase 2).
    ///
    /// This creates a new BQ storage and quantizes all existing vectors.
    /// Time complexity: O(n × d) where n = vector count, d = dimension.
    ///
    /// # Arguments
    ///
    /// * `storage` - The F32 vector storage containing vectors to quantize.
    ///
    /// # Errors
    ///
    /// Returns error if dimension is not divisible by 8.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(128);
    /// let mut storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::new(config, &storage).unwrap();
    ///
    /// // Insert some vectors
    /// index.insert(&vec![1.0f32; 128], &mut storage).unwrap();
    ///
    /// // Enable BQ later
    /// index.enable_bq(&storage).unwrap();
    /// assert!(index.has_bq());
    /// ```
    pub fn enable_bq(&mut self, storage: &VectorStorage) -> Result<(), GraphError> {
        let dimension = self.config.dimensions as usize;

        if dimension % 8 != 0 {
            return Err(GraphError::InvalidConfig(format!(
                "dimension must be divisible by 8 for binary quantization, got {dimension}"
            )));
        }

        let mut bq_storage =
            BinaryVectorStorage::new(dimension).map_err(|e| GraphError::Storage(e.to_string()))?;

        // Quantize all existing non-deleted vectors
        for node in &self.nodes {
            if node.deleted != 0 {
                // Insert placeholder for deleted nodes to maintain ID alignment
                let zeros = vec![0u8; dimension / 8];
                bq_storage
                    .insert_raw(&zeros)
                    .map_err(|e| GraphError::Storage(e.to_string()))?;
                continue;
            }

            let vector = storage.get_vector(node.vector_id);
            let bv = BinaryVector::quantize(&vector)
                .map_err(|e| GraphError::Quantization(e.to_string()))?;
            bq_storage
                .insert(&bv)
                .map_err(|e| GraphError::Storage(e.to_string()))?;
        }

        self.bq_storage = Some(bq_storage);
        Ok(())
    }

    /// Returns true if binary quantization is enabled.
    #[must_use]
    #[inline]
    pub fn has_bq(&self) -> bool {
        self.bq_storage.is_some()
    }

    /// Returns a reference to the BQ storage, if enabled.
    #[must_use]
    #[inline]
    pub fn bq_storage(&self) -> Option<&BinaryVectorStorage> {
        self.bq_storage.as_ref()
    }

    /// Generates a random level for a new node.
    ///
    /// Formula: `floor(-ln(uniform(0,1)) * m_l)`
    /// Clamped to `max_level` (e.g. 16) to prevent memory explosion.
    #[must_use]
    pub fn get_random_level(&mut self) -> u8 {
        // Generate uniform(0, 1)
        let r: f32 = self.rng.random_range(f32::EPSILON..=1.0);
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
            deleted: 0, // Live node (v0.3.0)
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

    // ============================================================================
    // Metadata API (v0.6.0 - RFC-002)
    // ============================================================================

    /// Returns an immutable reference to the metadata store.
    ///
    /// The metadata store contains key-value pairs attached to vectors.
    /// Use this method to query metadata without modifying it.
    ///
    /// # Thread Safety
    ///
    /// `MetadataStore` is `Send + Sync` when all contained types are `Send + Sync`.
    /// `String` and `MetadataValue` are both `Send + Sync`, so this is safe.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(4);
    /// let storage = VectorStorage::new(&config, None);
    /// let index = HnswIndex::new(config, &storage).unwrap();
    ///
    /// assert!(index.metadata().is_empty());
    /// ```
    #[must_use]
    pub fn metadata(&self) -> &MetadataStore {
        &self.metadata
    }

    /// Returns a mutable reference to the metadata store.
    ///
    /// Use this method to modify metadata attached to vectors.
    ///
    /// # Thread Safety
    ///
    /// Concurrent modification requires external synchronization (e.g., `Mutex`),
    /// matching the existing pattern for other `HnswIndex` operations.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::metadata::MetadataValue;
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(4);
    /// let storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::new(config, &storage).unwrap();
    ///
    /// index.metadata_mut()
    ///     .insert(1, "category", MetadataValue::String("books".into()))
    ///     .unwrap();
    ///
    /// assert!(index.metadata().has_key(1, "category"));
    /// ```
    pub fn metadata_mut(&mut self) -> &mut MetadataStore {
        &mut self.metadata
    }

    /// Inserts a vector with metadata atomically (v0.6.0 - RFC-002).
    ///
    /// This method validates metadata BEFORE inserting the vector, ensuring
    /// that either both the vector and metadata are stored, or neither is.
    ///
    /// # Arguments
    ///
    /// * `storage` - The vector storage to insert into.
    /// * `vector` - The vector data (must match configured dimensions).
    /// * `metadata` - Key-value metadata pairs to attach to the vector.
    ///
    /// # Returns
    ///
    /// The assigned `VectorId` on success.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::MetadataValidation` if metadata fails validation:
    /// - More than 64 keys
    /// - Key longer than 256 bytes
    /// - String value longer than 64KB
    /// - String array with more than 1024 elements
    /// - Invalid key format (must be alphanumeric + underscore)
    /// - Invalid float (NaN or Infinity)
    ///
    /// On error, the index remains unchanged (no partial state).
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::metadata::MetadataValue;
    /// use edgevec::storage::VectorStorage;
    /// use std::collections::HashMap;
    ///
    /// let config = HnswConfig::new(4);
    /// let mut storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::new(config, &storage).unwrap();
    ///
    /// let mut metadata = HashMap::new();
    /// metadata.insert("category".to_string(), MetadataValue::String("books".into()));
    /// metadata.insert("price".to_string(), MetadataValue::Float(29.99));
    ///
    /// let vector_id = index.insert_with_metadata(
    ///     &mut storage,
    ///     &[1.0, 2.0, 3.0, 4.0],
    ///     metadata,
    /// ).unwrap();
    ///
    /// // Note: VectorId is u64, but metadata uses u32 IDs
    /// #[allow(clippy::cast_possible_truncation)]
    /// let meta_id = vector_id.0 as u32;
    /// assert!(index.metadata().has_key(meta_id, "category"));
    /// ```
    pub fn insert_with_metadata(
        &mut self,
        storage: &mut VectorStorage,
        vector: &[f32],
        metadata: HashMap<String, MetadataValue>,
    ) -> Result<VectorId, GraphError> {
        use crate::metadata::validation::{validate_key_value, MAX_KEYS_PER_VECTOR};

        // Step 1: Validate metadata BEFORE any mutation
        // Check key count limit
        if metadata.len() > MAX_KEYS_PER_VECTOR {
            return Err(GraphError::MetadataValidation(MetadataError::TooManyKeys {
                vector_id: 0, // Unknown yet
                count: metadata.len(),
                max: MAX_KEYS_PER_VECTOR,
            }));
        }

        // Validate each key-value pair
        for (key, value) in &metadata {
            validate_key_value(key, value)?;
        }

        // Step 2: Insert vector (this is atomic — either succeeds or fails)
        let vector_id = self.insert(vector, storage)?;

        // Step 3: Store metadata for the newly inserted vector
        // This cannot fail because we pre-validated everything
        // Note: VectorId is u64 but MetadataStore uses u32. In practice,
        // vector IDs won't exceed u32::MAX (4B vectors).
        #[allow(clippy::cast_possible_truncation)]
        let metadata_id = vector_id.0 as u32;

        for (key, value) in metadata {
            // We've already validated, so insert should succeed
            self.metadata
                .insert(metadata_id, &key, value)
                .expect("pre-validated metadata should not fail");
        }

        Ok(vector_id)
    }

    // ============================================================================
    // Filtered Search API (W26.2.3 - RFC-002)
    // ============================================================================

    /// Search with post-filtering using metadata expressions.
    ///
    /// Performs a vector similarity search and filters results based on a
    /// metadata filter expression. Uses adaptive overfetch to ensure enough
    /// results pass the filter.
    ///
    /// # Arguments
    ///
    /// * `storage` - The vector storage
    /// * `query` - The query vector
    /// * `filter` - Filter expression (e.g., `"category = \"books\" AND price < 100"`)
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// Up to `k` results that pass the filter, sorted by distance (ascending).
    /// May return fewer than `k` results if not enough vectors pass the filter.
    ///
    /// # Errors
    ///
    /// * `GraphError::FilterParse` - Invalid filter syntax
    /// * `GraphError::FilterEval` - Filter evaluation failed
    /// * Other `GraphError` variants from underlying search
    ///
    /// # Algorithm (RFC-002 §3.2)
    ///
    /// 1. Parse filter expression
    /// 2. Use default selectivity = 0.50 (refined in W26.3.1)
    /// 3. Calculate overfetch_factor = min(10, max(2, 1 / selectivity))
    /// 4. Search with `k * overfetch_factor` candidates
    /// 5. Post-filter results using metadata evaluation
    /// 6. Return top-k passing results
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex};
    /// use edgevec::storage::VectorStorage;
    /// use edgevec::metadata::MetadataValue;
    /// use std::collections::HashMap;
    ///
    /// let config = HnswConfig::new(4);
    /// let mut storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::new(config, &storage).unwrap();
    ///
    /// // Insert vectors with metadata
    /// let mut meta = HashMap::new();
    /// meta.insert("category".to_string(), MetadataValue::String("books".into()));
    /// meta.insert("price".to_string(), MetadataValue::Integer(25));
    /// let _ = index.insert_with_metadata(&mut storage, &[1.0, 0.0, 0.0, 0.0], meta);
    ///
    /// // Search with filter
    /// let results = index.search_filtered(
    ///     &storage,
    ///     &[1.0, 0.0, 0.0, 0.0],
    ///     "category = \"books\" AND price < 100",
    ///     5,
    /// ).unwrap();
    /// ```
    pub fn search_filtered(
        &self,
        storage: &VectorStorage,
        query: &[f32],
        filter: &str,
        k: usize,
    ) -> Result<Vec<(VectorId, f32)>, GraphError> {
        use crate::filter::{
            estimate_filter_selectivity, evaluate, overfetch_from_selectivity, parse,
        };

        // Step 1: Parse filter expression
        let expr = parse(filter).map_err(|e| GraphError::FilterParse(e.to_string()))?;

        // Step 2: Estimate selectivity using heuristics (W26.3.1)
        let selectivity = estimate_filter_selectivity(&expr);

        // Step 3: Calculate overfetch factor from selectivity
        let overfetch_factor = overfetch_from_selectivity(selectivity);

        // Step 4: Search with overfetched k
        let overfetched_k = k.saturating_mul(overfetch_factor);
        let candidates = self.search(query, overfetched_k, storage)?;

        // Step 5: Post-filter results
        let mut passing_results = Vec::with_capacity(k);

        for result in candidates {
            // Get metadata for this vector
            #[allow(clippy::cast_possible_truncation)]
            let metadata_id = result.vector_id.0 as u32;

            // Get all metadata for this vector (empty HashMap if none)
            let metadata = self
                .metadata
                .get_all(metadata_id)
                .cloned()
                .unwrap_or_default();

            // Evaluate filter
            match evaluate(&expr, &metadata) {
                Ok(true) => {
                    passing_results.push((result.vector_id, result.distance));
                    if passing_results.len() >= k {
                        break;
                    }
                }
                Ok(false) => {
                    // Filter didn't match, skip
                }
                Err(e) => {
                    // Filter evaluation error (e.g., type mismatch)
                    // Per RFC-002: treat as filter failure, skip this result
                    // In production, we might want to log this
                    log::debug!(
                        "Filter evaluation failed for vector {}: {}",
                        result.vector_id.0,
                        e
                    );
                }
            }
        }

        // Step 6: Return results (already sorted by distance from search)
        Ok(passing_results)
    }

    // ============================================================================
    // Soft Delete API (W16.2 - RFC-001)
    // ============================================================================

    /// Get mutable reference to a node by VectorId.
    ///
    /// # Complexity
    ///
    /// * Time: O(n) linear scan
    /// * Space: O(1)
    fn get_node_mut(&mut self, vector_id: VectorId) -> Result<&mut HnswNode, GraphError> {
        self.nodes
            .iter_mut()
            .find(|n| n.vector_id == vector_id)
            .ok_or(GraphError::InvalidVectorId)
    }

    /// Get immutable reference to a node by VectorId.
    ///
    /// # Complexity
    ///
    /// * Time: O(n) linear scan
    /// * Space: O(1)
    fn get_node_by_vector_id(&self, vector_id: VectorId) -> Result<&HnswNode, GraphError> {
        self.nodes
            .iter()
            .find(|n| n.vector_id == vector_id)
            .ok_or(GraphError::InvalidVectorId)
    }

    /// Mark a vector as deleted (soft delete).
    ///
    /// The vector remains in the graph for routing but is excluded from
    /// search results. Space is reclaimed via `compact()`.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector to delete
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Vector was deleted
    /// * `Ok(false)` - Vector was already deleted
    /// * `Err(InvalidVectorId)` - Vector ID not found
    ///
    /// # Complexity
    ///
    /// * Time: **O(n)** for VectorId → Node lookup via linear scan,
    ///   plus O(1) for setting the deleted byte
    /// * Space: O(1)
    ///
    /// **Note:** The O(n) lookup is a known limitation in v0.3.0.
    /// A HashMap<VectorId, NodeId> index could provide O(1) lookup
    /// but is deferred to v0.4.0 to avoid scope creep.
    ///
    /// # Thread Safety (RFC-001 Design)
    ///
    /// This method requires `&mut self`, which means Rust's borrow checker
    /// prevents concurrent access at compile time. This is intentional:
    ///
    /// * Search (`&self`) and delete (`&mut self`) cannot run concurrently
    /// * No atomics or locks needed - safety is enforced by the type system
    /// * See RFC-001 Section 6.4 "Design Decisions" for rationale
    ///
    /// # Persistence
    ///
    /// **IMPORTANT:** Delete operations are in-memory only until `save()` is called.
    /// If the process crashes before `save()`, the delete will be lost.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let deleted = index.soft_delete(VectorId(42))?;
    /// assert!(deleted);
    /// assert!(index.is_deleted(VectorId(42))?);
    /// ```
    pub fn soft_delete(&mut self, vector_id: VectorId) -> Result<bool, GraphError> {
        let node = self.get_node_mut(vector_id)?;

        if node.deleted != 0 {
            return Ok(false); // Already deleted
        }

        node.deleted = 1;
        self.deleted_count += 1;

        // RFC-002 §2.3: Remove metadata when vector is soft-deleted
        // Note: VectorId is u64 but MetadataStore uses u32. In practice,
        // vector IDs won't exceed u32::MAX (4B vectors).
        #[allow(clippy::cast_possible_truncation)]
        let metadata_id = vector_id.0 as u32;
        self.metadata.delete_all(metadata_id);

        Ok(true)
    }

    /// Delete multiple vectors in a single operation
    ///
    /// **[C5 FIX] Two-Phase Implementation:**
    /// 1. Pre-validation: Check all IDs exist and are not already deleted
    /// 2. Execution: Apply deletions (guaranteed to succeed after validation)
    ///
    /// This prevents partial failures from leaving the index in an inconsistent state.
    ///
    /// **[C2 FIX] Duplicate Handling:**
    /// Duplicate IDs in the input are automatically deduplicated. Only the first
    /// occurrence is processed, duplicates are counted in `total` but not `unique_count`.
    ///
    /// **[M2 FIX] Memory Bounds:**
    /// Maximum batch size is capped at 10 million IDs to prevent memory exhaustion.
    ///
    /// # Arguments
    /// * `ids` - Slice of VectorId values to delete (duplicates allowed)
    ///
    /// # Returns
    /// * `BatchDeleteResult` with counts and detailed errors
    ///
    /// # Complexity
    /// * Time: O(N × M) where N = unique IDs, M = index size (for ID lookup)
    /// * Space: O(N) for deduplication and validation structures
    ///
    /// # Example
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(4);
    /// let mut storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::new(config, &storage).unwrap();
    ///
    /// // Insert some vectors
    /// for i in 0..10 {
    ///     index.insert(&vec![i as f32; 4], &mut storage).unwrap();
    /// }
    ///
    /// // Batch delete
    /// let ids = vec![VectorId(1), VectorId(3), VectorId(5)];
    /// let result = index.soft_delete_batch(&ids);
    ///
    /// assert_eq!(result.deleted, 3);
    /// assert_eq!(result.total, 3);
    /// assert!(result.all_valid());
    /// ```
    pub fn soft_delete_batch(&mut self, ids: &[VectorId]) -> BatchDeleteResult {
        // [M2 FIX] Memory bounds check: cap at 10M IDs (~80MB allocation)
        const MAX_BATCH_SIZE: usize = 10_000_000;

        let mut result = BatchDeleteResult {
            deleted: 0,
            already_deleted: 0,
            invalid_ids: 0,
            total: ids.len(),
            unique_count: 0,
            errors: Vec::new(),
        };

        if ids.is_empty() {
            return result;
        }

        // [M2 FIX] Check memory bounds
        if ids.len() > MAX_BATCH_SIZE {
            // Mark all as errors
            result.invalid_ids = ids.len();
            result.errors.push(BatchDeleteError::InternalError(
                VectorId(0),
                format!(
                    "Batch size {} exceeds maximum {}",
                    ids.len(),
                    MAX_BATCH_SIZE
                ),
            ));
            return result;
        }

        // [C2 FIX] Phase 0: Deduplication
        // Use HashSet to track seen IDs and eliminate duplicates
        let mut seen = HashSet::with_capacity(ids.len().min(1024)); // Cap initial allocation
        let mut unique_ids = Vec::with_capacity(ids.len().min(1024));

        for &id in ids {
            if seen.insert(id) {
                unique_ids.push(id);
            }
        }

        result.unique_count = unique_ids.len();

        // [C5 FIX] Phase 1: Pre-validation
        // Check all unique IDs and categorize them BEFORE making any changes
        let estimated_errors = unique_ids.len() / 10; // Assume 10% error rate
        let mut valid_ids = Vec::with_capacity(unique_ids.len());
        let mut already_deleted_count = 0;

        // [m1 FIX] Pre-allocate error vector with estimated capacity
        result.errors = Vec::with_capacity(estimated_errors);

        for &id in &unique_ids {
            match self.is_deleted(id) {
                Ok(true) => {
                    // Already deleted - not an error, just skip
                    already_deleted_count += 1;
                    result.errors.push(BatchDeleteError::AlreadyDeleted(id));
                }
                Ok(false) => {
                    // Valid and not deleted - queue for deletion
                    valid_ids.push(id);
                }
                Err(_) => {
                    // ID not found
                    result.invalid_ids += 1;
                    result.errors.push(BatchDeleteError::NotFound(id));
                }
            }
        }

        result.already_deleted = already_deleted_count;

        // [C5 FIX] Phase 2: Execution
        // All IDs in valid_ids are guaranteed to exist and not be deleted
        // This phase should not fail
        for &id in &valid_ids {
            match self.soft_delete(id) {
                Ok(true) => result.deleted += 1,
                Ok(false) => {
                    // Should not happen after validation, but handle gracefully
                    result.already_deleted += 1;
                }
                Err(e) => {
                    // Should not happen after validation
                    result.errors.push(BatchDeleteError::InternalError(
                        id,
                        format!("Unexpected error after validation: {e:?}"),
                    ));
                }
            }
        }

        result
    }

    /// Delete multiple vectors with progress callback
    ///
    /// **[C3 FIX] Two-Phase Implementation:**
    /// Now delegates to `soft_delete_batch()` for validation, then reports progress
    /// during the execution phase. This ensures consistent behavior between variants.
    ///
    /// **[M3 FIX] Panic Safety:**
    /// If the callback panics, the operation aborts but the index remains in a valid state
    /// (no partial deletions). The panic will unwind past this function.
    ///
    /// Callback is invoked approximately every 10% of progress during execution phase.
    /// Useful for UI updates during large batch operations.
    ///
    /// # Arguments
    /// * `ids` - Slice of VectorId values to delete (duplicates allowed)
    /// * `callback` - Function called with (processed_unique, total_unique) counts
    ///
    /// # Complexity
    /// * Time: O(N × M) + O(N × C) where N = unique IDs, M = index size, C = callback cost
    /// * Space: O(N) for deduplication and validation
    ///
    /// # Example
    /// ```
    /// use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
    /// use edgevec::storage::VectorStorage;
    ///
    /// let config = HnswConfig::new(4);
    /// let mut storage = VectorStorage::new(&config, None);
    /// let mut index = HnswIndex::new(config, &storage).unwrap();
    ///
    /// // Insert vectors
    /// for i in 0..100 {
    ///     index.insert(&vec![i as f32; 4], &mut storage).unwrap();
    /// }
    ///
    /// // Batch delete with progress
    /// let ids: Vec<VectorId> = (1..=50).map(VectorId).collect();
    /// let result = index.soft_delete_batch_with_progress(&ids, |processed, total| {
    ///     println!("Progress: {}/{}", processed, total);
    /// });
    /// ```
    pub fn soft_delete_batch_with_progress<F>(
        &mut self,
        ids: &[VectorId],
        mut callback: F,
    ) -> BatchDeleteResult
    where
        F: FnMut(usize, usize),
    {
        // [C3 FIX] Use the main two-phase implementation for consistency
        // We'll perform validation first (Phase 0-1), then execution with progress (Phase 2)

        // Phase 0-1: Deduplication and validation (same as soft_delete_batch)
        const MAX_BATCH_SIZE: usize = 10_000_000;

        let mut result = BatchDeleteResult {
            deleted: 0,
            already_deleted: 0,
            invalid_ids: 0,
            total: ids.len(),
            unique_count: 0,
            errors: Vec::new(),
        };

        if ids.is_empty() {
            callback(0, 0);
            return result;
        }

        if ids.len() > MAX_BATCH_SIZE {
            result.invalid_ids = ids.len();
            result.errors.push(BatchDeleteError::InternalError(
                VectorId(0),
                format!(
                    "Batch size {} exceeds maximum {}",
                    ids.len(),
                    MAX_BATCH_SIZE
                ),
            ));
            return result;
        }

        // Deduplication
        let mut seen = HashSet::with_capacity(ids.len().min(1024));
        let mut unique_ids = Vec::with_capacity(ids.len().min(1024));

        for &id in ids {
            if seen.insert(id) {
                unique_ids.push(id);
            }
        }

        result.unique_count = unique_ids.len();

        // Pre-validation
        let estimated_errors = unique_ids.len() / 10;
        let mut valid_ids = Vec::with_capacity(unique_ids.len());
        let mut already_deleted_count = 0;
        result.errors = Vec::with_capacity(estimated_errors);

        for &id in &unique_ids {
            match self.is_deleted(id) {
                Ok(true) => {
                    already_deleted_count += 1;
                    result.errors.push(BatchDeleteError::AlreadyDeleted(id));
                }
                Ok(false) => {
                    valid_ids.push(id);
                }
                Err(_) => {
                    result.invalid_ids += 1;
                    result.errors.push(BatchDeleteError::NotFound(id));
                }
            }
        }

        result.already_deleted = already_deleted_count;

        // [C3 FIX] Phase 2: Execution with progress callbacks
        // Calculate progress interval (~10% increments, minimum 1)
        let total_to_process = valid_ids.len();
        let interval = (total_to_process / 10).max(1);
        let mut last_callback = 0;

        for (i, &id) in valid_ids.iter().enumerate() {
            match self.soft_delete(id) {
                Ok(true) => result.deleted += 1,
                Ok(false) => {
                    result.already_deleted += 1;
                }
                Err(e) => {
                    result.errors.push(BatchDeleteError::InternalError(
                        id,
                        format!("Unexpected error after validation: {e:?}"),
                    ));
                }
            }

            // Fire callback at ~10% intervals
            if i + 1 - last_callback >= interval || i + 1 == total_to_process {
                // [M3 FIX] Callback may panic - if it does, the operation aborts here
                // but the index state is valid (all deletions up to this point succeeded)
                callback(i + 1, total_to_process);
                last_callback = i + 1;
            }
        }

        result
    }

    /// Check if a vector is marked as deleted.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector to check
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Vector is deleted
    /// * `Ok(false)` - Vector is live
    /// * `Err(InvalidVectorId)` - Vector ID not found
    ///
    /// # Complexity
    ///
    /// * Time: O(n) for lookup + O(1) for check
    /// * Space: O(1)
    pub fn is_deleted(&self, vector_id: VectorId) -> Result<bool, GraphError> {
        let node = self.get_node_by_vector_id(vector_id)?;
        Ok(node.deleted != 0)
    }

    /// Get the count of deleted (tombstoned) vectors.
    ///
    /// # Returns
    ///
    /// Number of vectors marked as deleted.
    #[must_use]
    pub fn deleted_count(&self) -> usize {
        self.deleted_count
    }

    /// Get the ratio of deleted vectors to total vectors.
    ///
    /// # Returns
    ///
    /// A value between 0.0 and 1.0 representing the tombstone ratio.
    /// Returns 0.0 if the index is empty.
    #[must_use]
    pub fn tombstone_ratio(&self) -> f64 {
        let total = self.node_count();
        if total == 0 {
            return 0.0;
        }
        self.deleted_count as f64 / total as f64
    }

    /// Get count of live (non-deleted) vectors.
    ///
    /// # Returns
    ///
    /// Number of vectors that are not marked as deleted.
    #[must_use]
    pub fn live_count(&self) -> usize {
        self.node_count().saturating_sub(self.deleted_count)
    }

    /// Maximum multiplier for adjusted_k to prevent excessive over-fetching.
    ///
    /// At 90%+ tombstones, we cap at 10x the original k to bound memory usage.
    /// Beyond this ratio, compaction should be triggered.
    pub const MAX_ADJUSTED_K_MULTIPLIER: usize = 10;

    /// Calculate adjusted k to compensate for tombstones.
    ///
    /// When the index has deleted vectors, we over-fetch to ensure
    /// we can return k live results after filtering. This method
    /// calculates how many candidates to fetch internally so that
    /// after filtering out deleted vectors, we likely have k results.
    ///
    /// # Formula
    ///
    /// `adjusted_k = k * total / live`
    ///
    /// This is equivalent to `k / (1 - tombstone_ratio)` but uses
    /// integer arithmetic for precision.
    ///
    /// # Caps
    ///
    /// The result is capped at `k * MAX_ADJUSTED_K_MULTIPLIER` (default 10x)
    /// to prevent excessive fetching at very high tombstone ratios.
    ///
    /// # Examples
    ///
    /// * 0% tombstones: k = k (no adjustment)
    /// * 10% tombstones: k → ~1.11x
    /// * 30% tombstones: k → ~1.43x
    /// * 50% tombstones: k → 2x
    /// * 90%+ tombstones: k → 10x (capped)
    ///
    /// # Thread Safety
    ///
    /// This method reads `deleted_count` and `node_count()` which may change
    /// if the index is mutated. Per RFC-001, the API uses `&mut self` for
    /// mutations, so concurrent read+write is prevented by Rust's borrow checker.
    /// The design accepts eventual consistency for soft delete semantics.
    ///
    /// # Arguments
    ///
    /// * `k` - The requested number of results
    ///
    /// # Returns
    ///
    /// The adjusted k value to use for internal search operations.
    /// This value is always >= k (unless capped by live_count).
    #[must_use]
    pub fn adjusted_k(&self, k: usize) -> usize {
        // Fast path: no tombstones
        if self.deleted_count == 0 {
            return k;
        }

        let total = self.node_count();
        let live = self.live_count();

        // Edge case: all deleted
        // This also prevents division by zero in the calculation below.
        if live == 0 {
            return k; // Will return empty results anyway
        }

        // Integer arithmetic: adjusted = k * total / live
        // Use saturating ops to prevent overflow
        let adjusted = k.saturating_mul(total) / live;

        // Cap at MAX_ADJUSTED_K_MULTIPLIER to prevent excessive over-fetching
        // Note: We don't cap at total because adjusted_k controls the internal
        // search effort (ef parameter), not the final result count.
        let max_by_multiplier = k.saturating_mul(Self::MAX_ADJUSTED_K_MULTIPLIER);
        adjusted.min(max_by_multiplier)
    }

    /// Marks a vector as deleted in the storage (legacy API).
    ///
    /// **DEPRECATED:** Use `soft_delete()` instead for RFC-001 compliant soft delete.
    /// This method delegates to storage and does not update `deleted_count`.
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
    #[deprecated(since = "0.3.0", note = "Use soft_delete() instead")]
    pub fn delete_in_storage(&self, id: VectorId, storage: &mut VectorStorage) -> bool {
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

    // ========================================================================
    // Compaction API (W16.4 - RFC-001)
    // ========================================================================

    /// Check if compaction is recommended.
    ///
    /// Returns `true` if the tombstone ratio exceeds the configured threshold
    /// (default 30%). When this returns true, calling `compact()` is
    /// recommended to reclaim space and maintain search performance.
    ///
    /// # Thread Safety
    ///
    /// This method is **not thread-safe**. `HnswIndex` is `!Send` and must be
    /// accessed from a single thread. For concurrent use, create separate index
    /// instances (e.g., via Web Workers in WASM).
    ///
    /// # Example
    ///
    /// ```ignore
    /// if index.needs_compaction() {
    ///     let (new_index, new_storage, result) = index.compact(&storage)?;
    ///     index = new_index;
    ///     storage = new_storage;
    /// }
    /// ```
    #[must_use]
    pub fn needs_compaction(&self) -> bool {
        self.tombstone_ratio() > self.compaction_threshold
    }

    /// Set the compaction threshold.
    ///
    /// When `tombstone_ratio()` exceeds this value, `needs_compaction()`
    /// returns true.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Tombstone ratio threshold (0.01 to 0.99)
    ///
    /// # Default
    ///
    /// Default is 0.3 (30%). Lower values trigger compaction more often
    /// but maintain better search performance.
    ///
    /// # Clamping
    ///
    /// Values outside [0.01, 0.99] are clamped to that range.
    pub fn set_compaction_threshold(&mut self, ratio: f64) {
        self.compaction_threshold = ratio.clamp(0.01, 0.99);
    }

    /// Get the current compaction threshold.
    ///
    /// # Returns
    ///
    /// The ratio at which `needs_compaction()` returns true.
    #[must_use]
    pub fn compaction_threshold(&self) -> f64 {
        self.compaction_threshold
    }

    /// Get a compaction warning message if compaction is recommended.
    ///
    /// Returns `Some(message)` if the tombstone ratio exceeds the threshold,
    /// or `None` if compaction is not needed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(warning) = index.compaction_warning() {
    ///     log::warn!("{}", warning);
    /// }
    /// ```
    #[must_use]
    pub fn compaction_warning(&self) -> Option<String> {
        if self.needs_compaction() {
            Some(format!(
                "Compaction recommended: tombstone ratio {:.1}% exceeds threshold {:.1}%. \
                 Call compact() to rebuild index without tombstones.",
                self.tombstone_ratio() * 100.0,
                self.compaction_threshold * 100.0
            ))
        } else {
            None
        }
    }

    /// Compact the index by rebuilding without tombstones.
    ///
    /// This operation creates a NEW index and NEW storage containing only
    /// live (non-deleted) vectors. The original index and storage are NOT
    /// modified, allowing the caller to replace them atomically.
    ///
    /// # Important: Vector ID Remapping
    ///
    /// Due to storage design constraints, vector IDs are remapped during
    /// compaction. New IDs are assigned sequentially starting from 1.
    /// If you need to track the mapping, use the returned index to query
    /// by position or content.
    ///
    /// # Returns
    ///
    /// Returns `(new_index, new_storage, result)` tuple. The caller MUST
    /// replace BOTH their index and storage references:
    ///
    /// ```ignore
    /// let (new_index, new_storage, result) = old_index.compact(&old_storage)?;
    /// println!("Removed {} tombstones in {}ms", result.tombstones_removed, result.duration_ms);
    /// // Now use new_index and new_storage, old ones will be dropped
    /// index = new_index;
    /// storage = new_storage;
    /// ```
    ///
    /// # Algorithm
    ///
    /// 1. Collect all live vectors (non-deleted)
    /// 2. Create a new empty index and storage with the same config
    /// 3. Re-insert all live vectors using regular insert()
    /// 4. Return the new pair
    ///
    /// # Performance
    ///
    /// * Time: O(n log n) where n = live vector count
    /// * Space: 2x index size during compaction (temporary)
    ///
    /// # Memory Safety
    ///
    /// * Returns new pair — no storage/index mismatch possible
    /// * On failure, original index/storage unchanged (caller keeps refs)
    /// * Old index/storage are NOT modified — caller drops when ready
    ///
    /// # Warning
    ///
    /// This is a blocking operation. For WASM, consider running
    /// during idle time or on user action.
    pub fn compact(
        &self,
        storage: &VectorStorage,
    ) -> Result<(HnswIndex, VectorStorage, CompactionResult), GraphError> {
        // WASM-compatible timing
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let start_ms = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);

        let original_deleted = self.deleted_count;
        let original_total = self.node_count();

        // Fast path: no tombstones — return copy
        if original_deleted == 0 {
            // Clone manually since VectorStorage doesn't implement Clone
            let config = self.config.clone();
            let mut new_storage = VectorStorage::new(&config, None);
            let mut new_index = HnswIndex::new(config, &new_storage)?;
            new_index.compaction_threshold = self.compaction_threshold;

            // Re-insert all vectors (this is a rebuild, but preserves order)
            for node in &self.nodes {
                let vec = storage.get_vector(node.vector_id);
                new_index.insert(&vec, &mut new_storage)?;
            }

            return Ok((
                new_index,
                new_storage,
                CompactionResult {
                    tombstones_removed: 0,
                    new_size: original_total,
                    duration_ms: 0,
                },
            ));
        }

        // Collect live vectors' data
        let live_vectors: Vec<Vec<f32>> = self
            .nodes
            .iter()
            .filter(|node| node.deleted == 0)
            .map(|node| {
                let vec = storage.get_vector(node.vector_id);
                vec.into_owned()
            })
            .collect();

        let new_size = live_vectors.len();

        // Build new index AND new storage with same config
        let config = self.config.clone();
        let mut new_storage = VectorStorage::new(&config, None);
        let mut new_index = HnswIndex::new(config, &new_storage)?;

        // Copy compaction threshold from original
        new_index.compaction_threshold = self.compaction_threshold;

        // Re-insert all live vectors (IDs will be remapped)
        for vector in live_vectors {
            new_index.insert(&vector, &mut new_storage)?;
        }

        // Calculate duration based on target
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_millis() as u64;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| (p.now() - start_ms) as u64)
            .unwrap_or(0);

        Ok((
            new_index,
            new_storage,
            CompactionResult {
                tombstones_removed: original_deleted,
                new_size,
                duration_ms,
            },
        ))
    }

    /// Insert a vector with a specific ID (validation only).
    ///
    /// This method validates that the specified ID doesn't conflict with
    /// existing IDs, then delegates to the standard `insert()` method.
    ///
    /// **Important:** Due to storage design constraints where VectorIds must
    /// match storage slot indices, the returned VectorId is the one assigned
    /// by storage, NOT the requested ID. The `id` parameter is only used
    /// for duplicate validation.
    ///
    /// For actual ID preservation during compaction, the storage would need
    /// to support sparse ID assignment, which is not currently implemented.
    ///
    /// # Arguments
    ///
    /// * `id` - The specific VectorId to validate (not actually used)
    /// * `vector` - The vector data (must match configured dimensions)
    /// * `storage` - Mutable reference to vector storage
    ///
    /// # Returns
    ///
    /// The VectorId assigned by storage (sequential).
    ///
    /// # Errors
    ///
    /// * `InvalidVectorId` - If ID already exists in index or is the sentinel value
    /// * `DimensionMismatch` - If vector dimensions don't match config
    /// * `Storage` - If storage operation fails
    pub fn insert_with_id(
        &mut self,
        id: VectorId,
        vector: &[f32],
        storage: &mut VectorStorage,
    ) -> Result<VectorId, GraphError> {
        // Validate ID is not sentinel
        if id == VectorId::INVALID {
            return Err(GraphError::InvalidVectorId);
        }

        // Validate ID doesn't already exist
        if self.nodes.iter().any(|n| n.vector_id == id) {
            return Err(GraphError::InvalidVectorId);
        }

        // Validate dimensions
        if vector.len() != self.config.dimensions as usize {
            return Err(GraphError::DimensionMismatch {
                expected: self.config.dimensions as usize,
                actual: vector.len(),
            });
        }

        // Delegate to regular insert (which handles all the graph connection logic)
        self.insert(vector, storage)
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
        for l in &mut levels {
            *l = index.get_random_level();
        }

        // Level 0 should be most common
        #[allow(clippy::naive_bytecount)]
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
            let vectors: Vec<(u64, Vec<f32>)> =
                (1..=10).map(|i| (i as u64, vec![i as f32; 4])).collect();

            let mut progress_calls: Vec<(usize, usize)> = vec![];

            let result = index.batch_insert(
                vectors,
                &mut storage,
                Some(|current, total| {
                    progress_calls.push((current, total));
                }),
            );

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

            let result = index.batch_insert(
                vectors,
                &mut storage,
                Some(|current, total| {
                    progress_calls.push((current, total));
                }),
            );

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
                (1u64, vec![1.0, 2.0, 3.0, 4.0]),      // Valid
                (2u64, vec![f32::NAN, 2.0, 3.0, 4.0]), // NaN - skip
                (3u64, vec![3.0, 3.0, 3.0]),           // Wrong dim - skip
                (1u64, vec![4.0, 4.0, 4.0, 4.0]),      // Duplicate - skip
                (0u64, vec![5.0, 5.0, 5.0, 5.0]),      // Reserved ID - skip
                (4u64, vec![6.0, 6.0, 6.0, 6.0]),      // Valid
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

    // ============================================================
    // SOFT DELETE TESTS (W16.2 - RFC-001)
    // ============================================================

    mod delete_tests {
        use super::*;

        fn create_test_index() -> (HnswIndex, VectorStorage) {
            let config = HnswConfig::new(4);
            let storage = VectorStorage::new(&config, None);
            let index = HnswIndex::new(config, &storage).unwrap();
            (index, storage)
        }

        #[test]
        fn test_soft_delete_marks_node() {
            let (mut index, mut storage) = create_test_index();
            let vec = vec![1.0, 2.0, 3.0, 4.0];
            let id = index.insert(&vec, &mut storage).unwrap();

            assert!(!index.is_deleted(id).unwrap());
            assert!(index.soft_delete(id).unwrap());
            assert!(index.is_deleted(id).unwrap());
        }

        #[test]
        fn test_soft_delete_idempotent() {
            let (mut index, mut storage) = create_test_index();
            let vec = vec![1.0, 2.0, 3.0, 4.0];
            let id = index.insert(&vec, &mut storage).unwrap();

            assert!(index.soft_delete(id).unwrap()); // First: true
            assert!(!index.soft_delete(id).unwrap()); // Second: false
            assert_eq!(index.deleted_count(), 1); // Still 1
        }

        #[test]
        fn test_soft_delete_nonexistent_fails() {
            let (mut index, _storage) = create_test_index();
            let result = index.soft_delete(VectorId(999));
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), GraphError::InvalidVectorId));
        }

        #[test]
        fn test_deleted_count() {
            let (mut index, mut storage) = create_test_index();

            // Insert 3 vectors
            let id1 = index.insert(&[1.0, 0.0, 0.0, 0.0], &mut storage).unwrap();
            let id2 = index.insert(&[0.0, 1.0, 0.0, 0.0], &mut storage).unwrap();
            let _id3 = index.insert(&[0.0, 0.0, 1.0, 0.0], &mut storage).unwrap();

            assert_eq!(index.deleted_count(), 0);
            assert_eq!(index.node_count(), 3);

            // Delete 2
            index.soft_delete(id1).unwrap();
            index.soft_delete(id2).unwrap();

            assert_eq!(index.deleted_count(), 2);
            assert_eq!(index.live_count(), 1);
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn test_tombstone_ratio() {
            let (mut index, mut storage) = create_test_index();

            // Empty index
            assert_eq!(index.tombstone_ratio(), 0.0);

            // Insert 4 vectors
            let mut ids = Vec::new();
            #[allow(clippy::cast_precision_loss)]
            for i in 0..4 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            assert!((index.tombstone_ratio() - 0.0).abs() < f64::EPSILON);

            // Delete 1 of 4 = 25%
            index.soft_delete(ids[0]).unwrap();
            assert!((index.tombstone_ratio() - 0.25).abs() < 0.01);

            // Delete 2 of 4 = 50%
            index.soft_delete(ids[1]).unwrap();
            assert!((index.tombstone_ratio() - 0.50).abs() < 0.01);
        }

        #[test]
        fn test_is_deleted_nonexistent_fails() {
            let (index, _storage) = create_test_index();
            let result = index.is_deleted(VectorId(999));
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), GraphError::InvalidVectorId));
        }

        #[test]
        fn test_live_count() {
            let (mut index, mut storage) = create_test_index();

            // Insert 5
            let mut ids = Vec::new();
            for i in 0..5 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete 2
            index.soft_delete(ids[0]).unwrap();
            index.soft_delete(ids[1]).unwrap();

            assert_eq!(index.node_count(), 5);
            assert_eq!(index.deleted_count(), 2);
            assert_eq!(index.live_count(), 3);
        }

        #[test]
        fn test_get_node_by_vector_id_helper() {
            let (mut index, mut storage) = create_test_index();
            let id = index.insert(&[1.0, 2.0, 3.0, 4.0], &mut storage).unwrap();

            // Should find existing node
            let node = index.get_node_by_vector_id(id);
            assert!(node.is_ok());
            assert_eq!(node.unwrap().vector_id, id);

            // Should fail for non-existent
            let bad = index.get_node_by_vector_id(VectorId(999));
            assert!(bad.is_err());
        }

        #[test]
        fn test_deleted_field_is_zero_by_default() {
            let (mut index, mut storage) = create_test_index();
            let id = index.insert(&[1.0, 2.0, 3.0, 4.0], &mut storage).unwrap();

            let node = index.get_node_by_vector_id(id).unwrap();
            assert_eq!(node.deleted, 0);
        }

        #[test]
        fn test_deleted_field_set_to_one_after_delete() {
            let (mut index, mut storage) = create_test_index();
            let id = index.insert(&[1.0, 2.0, 3.0, 4.0], &mut storage).unwrap();

            index.soft_delete(id).unwrap();

            let node = index.get_node_by_vector_id(id).unwrap();
            assert_eq!(node.deleted, 1);
        }

        #[test]
        fn test_delete_invalid_vector_id_zero() {
            let (mut index, _storage) = create_test_index();
            // VectorId(0) is INVALID sentinel
            let result = index.soft_delete(VectorId(0));
            assert!(result.is_err());
        }

        // ============================================================
        // ADJUSTED K TESTS (W16.3)
        // ============================================================

        #[test]
        fn test_adjusted_k_no_tombstones() {
            let (index, _storage) = create_test_index();
            // No deletions -> k unchanged
            assert_eq!(index.adjusted_k(10), 10);
            assert_eq!(index.adjusted_k(1), 1);
            assert_eq!(index.adjusted_k(100), 100);
        }

        #[test]
        fn test_adjusted_k_with_tombstones() {
            let (mut index, mut storage) = create_test_index();

            // Insert 10 vectors
            let mut ids = Vec::new();
            for i in 0..10 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete 5 of 10 = 50% tombstones
            for id in ids.iter().take(5) {
                index.soft_delete(*id).unwrap();
            }

            // 50% tombstones → 2x multiplier
            // adjusted_k(10) = ceil(10 / 0.5) = 20
            let adjusted = index.adjusted_k(10);
            assert!(
                (18..=22).contains(&adjusted),
                "Expected ~20, got {adjusted}"
            );
        }

        #[test]
        fn test_adjusted_k_capped_at_10x() {
            let (mut index, mut storage) = create_test_index();

            // Insert 100 vectors
            let mut ids = Vec::new();
            for i in 0..100 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete 95 of 100 = 95% tombstones
            for id in ids.iter().take(95) {
                index.soft_delete(*id).unwrap();
            }

            // Should cap at 10x, not 20x
            let adjusted = index.adjusted_k(10);
            assert_eq!(adjusted, 100, "Should be capped at 10x (100)");
        }

        #[test]
        fn test_adjusted_k_10_percent_tombstones() {
            let (mut index, mut storage) = create_test_index();

            // Insert 10 vectors
            let mut ids = Vec::new();
            for i in 0..10 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete 1 of 10 = 10% tombstones
            index.soft_delete(ids[0]).unwrap();

            // 10% tombstones → 1.11x multiplier
            // adjusted_k(10) = ceil(10 / 0.9) = ceil(11.11) = 12
            let adjusted = index.adjusted_k(10);
            assert!(
                (11..=13).contains(&adjusted),
                "Expected ~12, got {adjusted}"
            );
        }

        // ============================================================
        // BOUNDARY VALUE TESTS (m3 fix)
        // ============================================================

        #[test]
        fn test_adjusted_k_boundary_zero_tombstones() {
            let (mut index, mut storage) = create_test_index();

            // Insert vectors but delete none (0% tombstones)
            for i in 0..10 {
                index.insert(&[i as f32; 4], &mut storage).unwrap();
            }

            // 0% tombstones → no adjustment
            assert_eq!(index.adjusted_k(5), 5);
            assert_eq!(index.adjusted_k(10), 10);
            assert!((index.tombstone_ratio() - 0.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_adjusted_k_boundary_50_percent() {
            let (mut index, mut storage) = create_test_index();

            let mut ids = Vec::new();
            for i in 0..10 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete exactly 5 of 10 = 50%
            for id in ids.iter().take(5) {
                index.soft_delete(*id).unwrap();
            }

            // 50% tombstones → 2x
            // adjusted_k(10) = 10 * 10 / 5 = 20
            let adjusted = index.adjusted_k(10);
            assert_eq!(adjusted, 20, "50% tombstones should give 2x");
            assert!((index.tombstone_ratio() - 0.5).abs() < 0.01);
        }

        #[test]
        fn test_adjusted_k_boundary_90_percent() {
            let (mut index, mut storage) = create_test_index();

            let mut ids = Vec::new();
            for i in 0..10 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete 9 of 10 = 90%
            for id in ids.iter().take(9) {
                index.soft_delete(*id).unwrap();
            }

            // 90% tombstones → 10x
            // adjusted_k(10) = 10 * 10 / 1 = 100
            // Capped at 10x multiplier = 100
            let adjusted = index.adjusted_k(10);
            assert_eq!(adjusted, 100, "90% tombstones should give 10x (capped)");
            assert!((index.tombstone_ratio() - 0.9).abs() < 0.01);
        }

        #[test]
        fn test_adjusted_k_boundary_all_deleted() {
            let (mut index, mut storage) = create_test_index();

            let mut ids = Vec::new();
            for i in 0..5 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete all = 100% tombstones
            for id in &ids {
                index.soft_delete(*id).unwrap();
            }

            // All deleted → returns k (search will return empty anyway)
            let adjusted = index.adjusted_k(10);
            assert_eq!(adjusted, 10, "All deleted should return original k");
            assert_eq!(index.live_count(), 0);
            assert!((index.tombstone_ratio() - 1.0).abs() < 0.01);
        }

        #[test]
        fn test_adjusted_k_large_k_small_index() {
            let (mut index, mut storage) = create_test_index();

            // Insert only 5 vectors
            let mut ids = Vec::new();
            for i in 0..5 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete 2 = 40% tombstones
            index.soft_delete(ids[0]).unwrap();
            index.soft_delete(ids[1]).unwrap();

            // Request k=100, but only 5 vectors in index
            // adjusted_k(100) = 100 * 5 / 3 ≈ 166
            // Capped at 10x = 1000
            // Note: adjusted_k controls internal effort, not final result count
            let adjusted = index.adjusted_k(100);
            assert!(
                (166..=168).contains(&adjusted),
                "Should compute ~166 for 40% tombstones, got {adjusted}"
            );
        }

        #[test]
        fn test_adjusted_k_uses_constant() {
            // Verify MAX_ADJUSTED_K_MULTIPLIER is used
            assert_eq!(HnswIndex::MAX_ADJUSTED_K_MULTIPLIER, 10);
        }

        #[test]
        fn test_adjusted_k_integer_precision() {
            // Test that integer arithmetic doesn't lose precision
            let (mut index, mut storage) = create_test_index();

            let mut ids = Vec::new();
            for i in 0..100 {
                let id = index.insert(&[i as f32; 4], &mut storage).unwrap();
                ids.push(id);
            }

            // Delete 33 of 100 = 33%
            for id in ids.iter().take(33) {
                index.soft_delete(*id).unwrap();
            }

            // adjusted_k(10) = 10 * 100 / 67 = 14.925... → 14
            let adjusted = index.adjusted_k(10);
            // Should be close to 15 (1.49x)
            assert!(
                (14..=16).contains(&adjusted),
                "33% tombstones: expected ~15, got {adjusted}"
            );
        }
    }
}

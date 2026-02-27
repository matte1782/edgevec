//! # EdgeVec
//!
//! High-performance embedded vector database for Browser, Node, and Edge.
//!
//! ## Current Status
//!
//! **PHASE 3: Implementation (Week 7 Complete)**
//!
//! **Status:** Week 7 Complete — Persistence Hardened
//!
//! Core vector storage, HNSW graph indexing, and full durability (WAL + Snapshots) are implemented and verified.
//!
//! ## Implemented Features
//!
//! - **HNSW Graph**: Full insertion and search implementation with heuristic optimization.
//! - **Vector Storage**: Contiguous memory layout for fast access.
//! - **Scalar Quantization (SQ8)**: 4x memory reduction (f32 -> u8) with high accuracy.
//! - **Durability**: Write-Ahead Log (WAL) with CRC32 checksums, crash recovery, and atomic snapshots.
//! - **Metrics**: L2 (Euclidean), Cosine, and Dot Product distance functions.
//!
//! ## Development Protocol
//!
//! `EdgeVec` follows a military-grade development protocol:
//!
//! 1. **Architecture Phase** — Design docs must be approved before planning
//! 2. **Planning Phase** — Roadmap must be approved before coding
//! 3. **Implementation Phase** — Weekly tasks must be approved before coding
//! 4. **All gates require `HOSTILE_REVIEWER` approval**
//!
//! ## Example
//!
//! ```rust
//! use edgevec::{HnswConfig, HnswIndex, Metric, VectorStorage};
//!
//! // 1. Create Config
//! let config = HnswConfig::new(128);
//!
//! // 2. Initialize Storage and Index
//! let mut storage = VectorStorage::new(&config, None);
//! let mut index = HnswIndex::new(config, &storage).expect("failed to create index");
//!
//! // 3. Insert Vectors
//! let vector = vec![0.5; 128];
//! let id = index.insert(&vector, &mut storage).expect("failed to insert");
//!
//! // 4. Search
//! let query = vec![0.5; 128];
//! let results = index.search(&query, 10, &storage).expect("failed to search");
//!
//! assert!(!results.is_empty());
//! assert_eq!(results[0].vector_id, id);
//! ```
//!
//! ## Persistence Example
//!
//! ```rust,no_run
//! use edgevec::{HnswConfig, HnswIndex, VectorStorage};
//! use edgevec::persistence::{write_snapshot, read_snapshot, MemoryBackend};
//!
//! // Create index and storage
//! let config = HnswConfig::new(128);
//! let mut storage = VectorStorage::new(&config, None);
//! let mut index = HnswIndex::new(config, &storage).expect("failed to create");
//!
//! // Save snapshot using storage backend
//! let mut backend = MemoryBackend::new();
//! write_snapshot(&index, &storage, &mut backend).expect("failed to save");
//!
//! // Load snapshot
//! let (loaded_index, loaded_storage) = read_snapshot(&backend).expect("failed to load");
//! ```
//!
//! ## Next Steps (Phase 5)
//!
//! 1. **Documentation**: Finalize API docs.
//! 2. **NPM Package**: Release to npm registry.
//! 3. **Performance**: Final tuning and benchmarks.
//!
//! ## Documentation
//!
//! - [Genesis Workflow](docs/GENESIS_WORKFLOW.md)
//! - [Agent Commands](.cursor/commands/README.md)
//! - [Supreme Rules](.cursorrules)

#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::pub_underscore_fields)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::must_use_candidate)]

/// Persistence and file format definitions.
pub mod persistence;

/// Unified error handling.
pub mod error;

/// Batch insertion API.
pub mod batch;

/// HNSW Graph implementation.
pub mod hnsw;

/// Distance metrics.
pub mod metric;

/// Vector storage.
pub mod storage;

/// WASM bindings (only compiled on wasm32 targets).
#[cfg(target_arch = "wasm32")]
pub mod wasm;

/// Quantization support.
pub mod quantization;

/// SIMD capability detection and runtime optimization.
pub mod simd;

/// Metadata storage for vector annotations.
pub mod metadata;

/// Filter expression parsing and evaluation.
pub mod filter;

/// Flat (brute-force) index for binary vectors.
pub mod flat;

/// Index implementations (FlatIndex, etc.).
pub mod index;

/// Sparse vector support for hybrid search.
#[cfg(feature = "sparse")]
pub mod sparse;

// =============================================================================
// Index Type Selection
// =============================================================================

/// Index type for vector search.
///
/// EdgeVec supports two index types with different performance characteristics:
///
/// | Index Type | Insert | Search (1M) | Recall | Best For |
/// |------------|--------|-------------|--------|----------|
/// | **Flat**   | O(1) ~1μs | O(n) ~5-10ms | 100% (exact) | Real-time apps, <1M vectors |
/// | **HNSW**   | O(log n) ~2ms | O(log n) ~2ms | 90-95% | Large datasets, batch insert |
///
/// # Example (Rust)
///
/// ```rust
/// use edgevec::{IndexType, HnswConfig, BinaryFlatIndex};
///
/// // Create a flat index for insert-heavy workloads
/// let flat = BinaryFlatIndex::new(1024);
///
/// // Create an HNSW index for large-scale search
/// let config = HnswConfig::new(1024);
/// let index_type = IndexType::Hnsw(config);
/// ```
#[derive(Debug, Clone)]
pub enum IndexType {
    /// Brute force search (O(1) insert, O(n) search).
    ///
    /// Use for:
    /// - Insert-heavy workloads (semantic caching)
    /// - Datasets < 1M vectors
    /// - When 100% recall (exact search) is required
    /// - When insert latency is critical (~1μs vs ~2ms for HNSW)
    Flat,

    /// HNSW graph index (O(log n) insert, O(log n) search).
    ///
    /// Use for:
    /// - Large datasets (>1M vectors)
    /// - Read-heavy workloads
    /// - When approximate nearest neighbors is acceptable
    Hnsw(HnswConfig),
}

impl IndexType {
    /// Create a Flat index type.
    #[must_use]
    pub fn flat() -> Self {
        IndexType::Flat
    }

    /// Create an HNSW index type with configuration for given dimensions.
    ///
    /// Uses default HNSW parameters (M=12, M0=24, ef_construction=100, ef_search=50).
    #[must_use]
    pub fn hnsw(dimensions: u32) -> Self {
        IndexType::Hnsw(HnswConfig::new(dimensions))
    }

    /// Create an HNSW index type with custom configuration.
    #[must_use]
    pub fn hnsw_with_config(config: HnswConfig) -> Self {
        IndexType::Hnsw(config)
    }

    /// Check if this is a Flat index.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        matches!(self, IndexType::Flat)
    }

    /// Check if this is an HNSW index.
    #[must_use]
    pub fn is_hnsw(&self) -> bool {
        matches!(self, IndexType::Hnsw(_))
    }
}

/// Hybrid search combining dense and sparse retrieval.
#[cfg(feature = "sparse")]
pub mod hybrid;

pub use batch::BatchInsertable;
pub use error::BatchError;
pub use flat::{BinaryFlatIndex, BinaryFlatIndexError, BinaryFlatSearchResult};
pub use hnsw::{BatchDeleteError, BatchDeleteResult, HnswConfig, HnswIndex, SearchResult};
pub use metric::Metric;

// Re-export IndexType (defined in this crate root)
// No `use` statement needed since it's already defined above
pub use persistence::ChunkedWriter;
pub use quantization::{BinaryQuantizer, QuantizedVector, QuantizerConfig, ScalarQuantizer};
pub use simd::{
    capabilities, detect_neon, select_backend, warn_if_suboptimal, SimdBackend, SimdCapabilities,
};
pub use storage::VectorStorage;

pub use index::{
    DistanceMetric, FlatIndex, FlatIndexConfig, FlatIndexError, FlatIndexHeader, FlatSearchResult,
    FLAT_INDEX_MAGIC, FLAT_INDEX_VERSION,
};

#[cfg(feature = "sparse")]
pub use sparse::{SparseError, SparseVector};

/// The crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the crate version string.
///
/// # Returns
///
/// The crate version string.
///
/// # Example
///
/// ```rust
/// let version = edgevec::version();
/// assert!(!version.is_empty());
/// ```
#[must_use]
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_not_empty() {
        assert!(!version().is_empty());
    }
}

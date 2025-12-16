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

/// WASM bindings.
pub mod wasm;

/// Quantization support.
pub mod quantization;

/// SIMD capability detection and runtime optimization.
pub mod simd;

/// Metadata storage for vector annotations.
pub mod metadata;

pub use batch::BatchInsertable;
pub use error::BatchError;
pub use hnsw::{BatchDeleteError, BatchDeleteResult, HnswConfig, HnswIndex, SearchResult};
pub use metric::Metric;
pub use persistence::ChunkedWriter;
pub use quantization::{BinaryQuantizer, QuantizedVector, QuantizerConfig, ScalarQuantizer};
pub use simd::{
    capabilities, detect_neon, select_backend, warn_if_suboptimal, SimdBackend, SimdCapabilities,
};
pub use storage::VectorStorage;

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

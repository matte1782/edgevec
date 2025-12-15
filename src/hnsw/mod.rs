//! HNSW module containing graph logic, configuration, and search.

/// Configuration types.
pub mod config;
/// Graph data structures.
pub mod graph;
/// Insertion algorithms.
pub mod insert;
/// Neighbor management.
pub mod neighbor;
/// Search algorithms.
pub mod search;

pub use config::HnswConfig;
pub use graph::{
    BatchDeleteError, BatchDeleteResult, CompactionResult, GraphError, HnswIndex, HnswNode, NodeId,
    VectorId, VectorProvider,
};
pub use neighbor::NeighborPool;
pub use search::{Candidate, SearchContext, SearchResult, Searcher};

/// Alias for `HnswIndex` to support legacy tests.
pub type HnswGraph = HnswIndex;

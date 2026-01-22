//! Hybrid search combining dense and sparse retrieval.
//!
//! This module provides fusion algorithms and search orchestration for
//! combining dense semantic search (HNSW) with sparse keyword search (BM25).
//!
//! # Fusion Methods
//!
//! Two fusion methods are supported:
//!
//! 1. **Reciprocal Rank Fusion (RRF)**: Combines ranks using
//!    `score(d) = sum(1 / (k + rank_i(d)))`. Robust, parameter-insensitive.
//!
//! 2. **Linear Combination**: Combines normalized scores using
//!    `score(d) = alpha * dense_score + (1 - alpha) * sparse_score`.
//!
//! # Example
//!
//! ```rust
//! use edgevec::hybrid::{rrf_fusion, FusionResult};
//!
//! // Dense search results: [(id, score), ...]
//! let dense = vec![(1, 0.95), (2, 0.80), (3, 0.75)];
//!
//! // Sparse search results: [(id, score), ...]
//! let sparse = vec![(2, 5.5), (4, 4.2), (1, 3.8)];
//!
//! // Fuse with RRF (k=60)
//! let fused = rrf_fusion(&dense, &sparse, 60, 10);
//!
//! // Results combine ranks from both lists
//! for result in &fused {
//!     println!("ID: {}, RRF Score: {}", result.id, result.score);
//! }
//! ```

mod fusion;
mod search;

pub use fusion::{linear_fusion, rrf_fusion, FusionMethod, FusionResult, RRF_DEFAULT_K};
pub use search::{HybridError, HybridSearchConfig, HybridSearchResult, HybridSearcher};

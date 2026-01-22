//! Index implementations for EdgeVec.
//!
//! This module provides different indexing strategies for vector search:
//!
//! - [`FlatIndex`]: Brute-force search for small datasets (<10k vectors)
//!   with 100% recall guarantee and O(1) insert.
//!
//! # Choosing an Index
//!
//! | Index | Best For | Recall | Insert | Search |
//! |-------|----------|--------|--------|--------|
//! | [`FlatIndex`] | <10k vectors | 100% | O(1) | O(n·d) |
//! | [`HnswIndex`](crate::hnsw::HnswIndex) | >10k vectors | ~95% | O(log n) | O(log n) |
//!
//! # Example
//!
//! ```rust
//! use edgevec::index::{FlatIndex, FlatIndexConfig, DistanceMetric};
//!
//! // Create a flat index for 128-dimensional vectors
//! let config = FlatIndexConfig::new(128)
//!     .with_metric(DistanceMetric::Cosine);
//! let mut index = FlatIndex::new(config);
//!
//! // Insert vectors (O(1) operation)
//! let id = index.insert(&[0.1; 128]).unwrap();
//!
//! // Get vector by ID
//! let vector = index.get(id).unwrap();
//! ```

mod flat;

pub use flat::{
    DistanceMetric, FlatIndex, FlatIndexConfig, FlatIndexError, FlatIndexHeader, FlatSearchResult,
    FLAT_INDEX_MAGIC, FLAT_INDEX_VERSION,
};

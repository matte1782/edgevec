//! Sparse vector support for hybrid search.
//!
//! This module provides sparse vector types and operations for combining
//! dense semantic embeddings with sparse keyword features (BM25, TF-IDF).
//!
//! # Feature Flags
//!
//! - `sparse` (default): Core sparse types and metrics
//!
//! # Example
//!
//! ```rust
//! use edgevec::sparse::{SparseVector, SparseError};
//!
//! // Create a sparse vector from sorted indices and values
//! let indices = vec![0, 5, 10];
//! let values = vec![0.5, 0.3, 0.2];
//! let sparse = SparseVector::new(indices, values, 100)?;
//!
//! // Compute dot product
//! let other = SparseVector::new(vec![5, 10], vec![0.4, 0.6], 100)?;
//! let dot = sparse.dot(&other);
//! # Ok::<(), SparseError>(())
//! ```

mod error;
mod metrics;
mod search; // Week 39: SparseSearcher implementation
mod storage;
mod vector;

pub use error::SparseError;
pub use metrics::{sparse_cosine, sparse_dot_product, sparse_norm};
pub use search::{SparseSearchResult, SparseSearcher}; // Week 39
pub use storage::{SparseId, SparseStorage, SPARSE_FORMAT_VERSION, SPARSE_MAGIC};
pub use vector::SparseVector;

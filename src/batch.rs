//! Batch insertion API for HNSW indexes.
//!
//! This module provides the [`BatchInsertable`] trait for efficient
//! insertion of multiple vectors in a single operation.
//!
//! # Performance
//!
//! Batch insertion provides **convenience features** rather than raw throughput gains:
//! - Single API call instead of loop
//! - Built-in progress tracking at ~10% intervals
//! - Best-effort semantics (partial success on non-fatal errors)
//! - Progress callback overhead: <1%
//!
//! **Note:** Throughput is equivalent to sequential insertion since both
//! use the same underlying HNSW algorithm. See `benches/batch_vs_sequential.rs`.
//!
//! # Example
//!
//! ```ignore
//! use edgevec::{HnswConfig, HnswIndex, VectorStorage, batch::BatchInsertable, error::BatchError};
//!
//! fn main() -> Result<(), BatchError> {
//!     // Create an HNSW index
//!     let config = HnswConfig::new(128);
//!     let mut storage = VectorStorage::new(&config, None);
//!     let mut index = HnswIndex::new(config, &storage).unwrap();
//!
//!     // Prepare vectors for batch insertion
//!     let vectors: Vec<(u64, Vec<f32>)> = vec![
//!         (1, vec![0.1; 128]),
//!         (2, vec![0.2; 128]),
//!     ];
//!
//!     // Batch insert with progress tracking
//!     let ids = index.batch_insert(vectors, &mut storage, Some(|inserted, total| {
//!         println!("Progress: {}/{}", inserted, total);
//!     }))?;
//!
//!     assert_eq!(ids.len(), 2);
//!     Ok(())
//! }
//! ```
//!
//! # Error Handling
//!
//! Batch insert uses **best-effort** semantics:
//! - Fatal errors (dimension mismatch on first vector, capacity exceeded) abort immediately
//! - Non-fatal errors (duplicates, invalid vectors mid-batch) are skipped
//! - Partial success is returned via `Ok(Vec<u64>)`
//!
//! See [`BatchError`](crate::error::BatchError) for error types.

use crate::error::BatchError;
use crate::storage::VectorStorage;

/// Trait for HNSW indexes supporting batch insertion.
///
/// This trait provides efficient bulk insertion of multiple vectors
/// in a single operation, with optional progress tracking.
///
/// # Performance
///
/// Batch insertion provides **convenience features** rather than raw throughput gains:
/// - Single API call instead of loop
/// - Built-in progress tracking at ~10% intervals
/// - Best-effort semantics (partial success on non-fatal errors)
/// - Progress callback overhead: <1%
///
/// **Note:** Throughput is equivalent to sequential insertion.
/// See `benches/batch_vs_sequential.rs` for benchmark data.
///
/// # Atomicity
///
/// Batch insertion is **not atomic**. It follows best-effort semantics:
/// - If a fatal error occurs (e.g., dimension mismatch on first vector),
///   the operation aborts immediately with `Err(BatchError)`
/// - If a non-fatal error occurs (e.g., duplicate ID mid-batch),
///   the problematic vector is skipped and insertion continues
/// - Returns `Ok(Vec<u64>)` containing IDs of successfully inserted vectors
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// use edgevec::{HnswConfig, HnswIndex, VectorStorage, batch::BatchInsertable};
///
/// fn example() -> Result<(), edgevec::error::BatchError> {
///     let config = HnswConfig::new(128);
///     let mut storage = VectorStorage::new(&config, None);
///     let mut index = HnswIndex::new(config, &storage).unwrap();
///
///     let vectors = vec![
///         (1, vec![1.0; 128]),
///         (2, vec![2.0; 128]),
///         (3, vec![3.0; 128]),
///     ];
///
///     let ids = index.batch_insert(vectors, &mut storage, None)?;
///     println!("Inserted {} vectors", ids.len());
///     Ok(())
/// }
/// ```
///
/// With progress callback:
///
/// ```ignore
/// use edgevec::{HnswConfig, HnswIndex, VectorStorage, batch::BatchInsertable};
///
/// fn example() -> Result<(), edgevec::error::BatchError> {
///     let config = HnswConfig::new(128);
///     let mut storage = VectorStorage::new(&config, None);
///     let mut index = HnswIndex::new(config, &storage).unwrap();
///     let vectors: Vec<(u64, Vec<f32>)> = vec![(1, vec![1.0; 128])];
///
///     let ids = index.batch_insert(vectors, &mut storage, Some(|n, total| {
///         if n % 1000 == 0 || n == total {
///             println!("Progress: {}/{} ({:.1}%)", n, total, (n * 100) as f32 / total as f32);
///         }
///     }))?;
///     Ok(())
/// }
/// ```
pub trait BatchInsertable {
    /// Insert multiple vectors in a single operation.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Iterator of `(u64, Vec<f32>)` pairs (ID and vector data) to insert
    /// * `storage` - Mutable reference to vector storage for actual insertion
    /// * `progress_callback` - Optional callback invoked periodically as `(inserted_count, total_count)`
    ///
    /// The progress callback is invoked:
    /// - At 0% (before starting)
    /// - Approximately every 10% of progress
    /// - At 100% (after completion)
    /// - Never more than ~11 times total (to minimize overhead)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u64>)` - Vector IDs of successfully inserted vectors (may be partial on non-fatal errors)
    /// * `Err(BatchError)` - Fatal error that aborted the operation
    ///
    /// # Errors
    ///
    /// Returns `Err(BatchError)` for fatal errors:
    /// - [`BatchError::DimensionMismatch`] - First vector has wrong dimensionality
    /// - [`BatchError::CapacityExceeded`] - Batch would exceed index capacity
    /// - [`BatchError::InternalError`] - HNSW invariant violated
    ///
    /// For non-fatal errors (duplicate IDs, invalid vectors mid-batch), the problematic
    /// vector is skipped and insertion continues. Check the returned vector length
    /// against input length to detect skipped vectors.
    ///
    /// # Performance Notes
    ///
    /// - Progress callbacks add ~2-5% overhead (hypothesis). Omit for maximum throughput.
    /// - Batch size of 1000-10000 vectors is optimal for most use cases.
    /// - Memory usage: collects iterator upfront (N vectors × dimension × 4 bytes).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edgevec::{HnswConfig, HnswIndex, VectorStorage, batch::BatchInsertable, error::BatchError};
    ///
    /// fn main() -> Result<(), BatchError> {
    ///     let config = HnswConfig::new(128);
    ///     let mut storage = VectorStorage::new(&config, None);
    ///     let mut index = HnswIndex::new(config, &storage).unwrap();
    ///
    ///     // Prepare 1000 vectors
    ///     let vectors: Vec<_> = (0..1000)
    ///         .map(|i| (i, vec![i as f32; 128]))
    ///         .collect();
    ///
    ///     // Batch insert without progress tracking
    ///     let ids = index.batch_insert(vectors, &mut storage, None)?;
    ///     assert_eq!(ids.len(), 1000);
    ///     Ok(())
    /// }
    /// ```
    fn batch_insert<I, F>(
        &mut self,
        vectors: I,
        storage: &mut VectorStorage,
        progress_callback: Option<F>,
    ) -> Result<Vec<u64>, BatchError>
    where
        I: IntoIterator<Item = (u64, Vec<f32>)>,
        F: FnMut(usize, usize);
}

// Re-export VectorId for convenience
pub use crate::hnsw::graph::VectorId;

//! WASM Bindings for EdgeVec.

use crate::error::EdgeVecError;
use crate::hnsw::{GraphError, HnswConfig, HnswIndex};
use crate::metadata::MetadataStore;
use crate::persistence::{chunking::ChunkIter, ChunkedWriter, PersistenceError};
use crate::storage::VectorStorage;
use js_sys::{Array, Float32Array, Function, Object, Reflect, Uint32Array, Uint8Array};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Once,
};
use wasm_bindgen::prelude::*;

mod batch;
mod iterator;
mod metadata;

pub use batch::{BatchInsertConfig, BatchInsertResult};
pub use iterator::PersistenceIterator;
pub use metadata::JsMetadataValue;

/// Interface to the JavaScript IndexedDB backend.
#[wasm_bindgen(module = "/src/js/storage.js")]
extern "C" {
    /// The IndexedDB backend class.
    #[wasm_bindgen(js_name = IndexedDbBackend)]
    pub type IndexedDbBackend;

    /// Write data to the named database file.
    #[wasm_bindgen(static_method_of = IndexedDbBackend, catch)]
    pub async fn write(name: &str, data: &[u8]) -> Result<(), JsValue>;

    /// Read data from the named database file.
    #[wasm_bindgen(static_method_of = IndexedDbBackend, catch)]
    pub async fn read(name: &str) -> Result<JsValue, JsValue>;
}

static INIT: Once = Once::new();

/// Initialize logging hooks.
#[wasm_bindgen]
pub fn init_logging() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
}

/// Configuration for EdgeVec database.
#[wasm_bindgen]
pub struct EdgeVecConfig {
    /// Vector dimensionality.
    pub dimensions: u32,
    m: Option<u32>,
    m0: Option<u32>,
    ef_construction: Option<u32>,
    ef_search: Option<u32>,
    metric: Option<String>,
}

#[wasm_bindgen]
impl EdgeVecConfig {
    /// Create a new configuration with required dimensions.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(dimensions: u32) -> EdgeVecConfig {
        EdgeVecConfig {
            dimensions,
            m: None,
            m0: None,
            ef_construction: None,
            ef_search: None,
            metric: None,
        }
    }

    /// Set M parameter (max connections per node in layers > 0).
    #[wasm_bindgen(setter)]
    pub fn set_m(&mut self, m: u32) {
        self.m = Some(m);
    }

    /// Set M0 parameter (max connections per node in layer 0).
    #[wasm_bindgen(setter)]
    pub fn set_m0(&mut self, m0: u32) {
        self.m0 = Some(m0);
    }

    /// Set ef_construction parameter.
    #[wasm_bindgen(setter)]
    pub fn set_ef_construction(&mut self, ef: u32) {
        self.ef_construction = Some(ef);
    }

    /// Set ef_search parameter.
    #[wasm_bindgen(setter)]
    pub fn set_ef_search(&mut self, ef: u32) {
        self.ef_search = Some(ef);
    }

    /// Set distance metric ("l2", "cosine", "dot").
    #[wasm_bindgen(setter)]
    pub fn set_metric(&mut self, metric: String) {
        self.metric = Some(metric);
    }
}

/// The main EdgeVec database handle.
///
/// This struct is serializable for persistence via `postcard`.
/// The `liveness` field is skipped as it is runtime state.
///
/// # Safety Note
///
/// This type derives `Deserialize` despite containing methods with `unsafe`.
/// The unsafe code (`save_stream`) is unrelated to deserialization and is safe
/// because it only extends lifetimes for iterator borrowing, controlled by the
/// `liveness` guard.
#[derive(Serialize, Deserialize)]
#[allow(clippy::unsafe_derive_deserialize)]
#[wasm_bindgen]
pub struct EdgeVec {
    #[allow(dead_code)]
    inner: HnswIndex,
    #[allow(dead_code)]
    storage: VectorStorage,
    /// Metadata store for attaching key-value pairs to vectors.
    #[serde(default)]
    metadata: MetadataStore,
    /// Safety guard for iterators (skipped during serialization).
    #[serde(skip, default = "default_liveness")]
    liveness: Arc<AtomicBool>,
}

/// Default initializer for `liveness` field during deserialization.
fn default_liveness() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(true))
}

impl Drop for EdgeVec {
    fn drop(&mut self) {
        // Signal to any active iterators that we are dead
        self.liveness.store(false, Ordering::Release);
    }
}

#[wasm_bindgen]
impl EdgeVec {
    /// Creates a new EdgeVec database.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid (e.g., unknown metric).
    #[wasm_bindgen(constructor)]
    pub fn new(config: &EdgeVecConfig) -> Result<EdgeVec, JsValue> {
        // [m1] Ensure logging is initialized
        INIT.call_once(|| {
            init_logging();
        });

        // Convert EdgeVecConfig to HnswConfig
        let metric_code = match config.metric.as_deref() {
            Some("cosine") => HnswConfig::METRIC_COSINE,
            Some("dot") => HnswConfig::METRIC_DOT_PRODUCT,
            Some("l2") | None => HnswConfig::METRIC_L2_SQUARED,
            Some(other) => {
                return Err(EdgeVecError::Validation(format!("Unknown metric: {other}")).into())
            }
        };

        let mut hnsw_config = HnswConfig::new(config.dimensions);
        if let Some(m) = config.m {
            hnsw_config.m = m;
        }
        if let Some(m0) = config.m0 {
            hnsw_config.m0 = m0;
        }
        if let Some(ef) = config.ef_construction {
            hnsw_config.ef_construction = ef;
        }
        if let Some(ef) = config.ef_search {
            hnsw_config.ef_search = ef;
        }
        hnsw_config.metric = metric_code;

        // Initialize storage (in-memory for now)
        let storage = VectorStorage::new(&hnsw_config, None);

        let index = HnswIndex::new(hnsw_config, &storage).map_err(EdgeVecError::from)?;

        Ok(EdgeVec {
            inner: index,
            storage,
            metadata: MetadataStore::new(),
            liveness: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Inserts a vector into the index.
    ///
    /// # Arguments
    ///
    /// * `vector` - A Float32Array containing the vector data.
    ///
    /// # Returns
    ///
    /// The assigned Vector ID (u32).
    ///
    /// # Errors
    ///
    /// Returns error if dimensions mismatch, vector contains NaNs, or ID overflows.
    #[wasm_bindgen]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn insert(&mut self, vector: Float32Array) -> Result<u32, JsValue> {
        let len = vector.length();
        if len != self.inner.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: self.inner.config.dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let vec = vector.to_vec();

        // Removed explicit iter().any() check for performance in Release mode
        // The check was adding ~20% overhead on O(N) iteration
        #[cfg(debug_assertions)]
        if vec.iter().any(|v| !v.is_finite()) {
            return Err(
                EdgeVecError::Validation("Vector contains non-finite values".to_string()).into(),
            );
        }

        let id = self
            .inner
            .insert(&vec, &mut self.storage)
            .map_err(EdgeVecError::from)?;

        // Safety: VectorId is u64, we cast to u32 as requested by API.
        if id.0 > u64::from(u32::MAX) {
            return Err(EdgeVecError::Validation("Vector ID overflowed u32".to_string()).into());
        }
        Ok(id.0 as u32)
    }

    /// Inserts a batch of vectors into the index (flat array format).
    ///
    /// **Note:** This is the legacy API. For the new API, use `insertBatch` which
    /// accepts an Array of Float32Array.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Flat Float32Array containing `count * dimensions` elements.
    /// * `count` - Number of vectors in the batch.
    ///
    /// # Returns
    ///
    /// A Uint32Array containing the assigned Vector IDs.
    ///
    /// # Errors
    ///
    /// Returns error if dimensions mismatch, vector contains NaNs, or ID overflows.
    #[wasm_bindgen(js_name = insertBatchFlat)]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn insert_batch_flat(
        &mut self,
        vectors: Float32Array,
        count: usize,
    ) -> Result<Uint32Array, JsValue> {
        let dim = self.inner.config.dimensions as usize;
        let expected_len = count * dim;

        if vectors.length() as usize != expected_len {
            return Err(EdgeVecError::Validation(format!(
                "Batch dimension mismatch: expected {} ({} * {}), got {}",
                expected_len,
                count,
                dim,
                vectors.length()
            ))
            .into());
        }

        let vec_data = vectors.to_vec();

        #[cfg(debug_assertions)]
        if vec_data.iter().any(|v| !v.is_finite()) {
            return Err(
                EdgeVecError::Validation("Vectors contain non-finite values".to_string()).into(),
            );
        }

        let mut ids = Vec::with_capacity(count);

        for i in 0..count {
            let start = i * dim;
            let end = start + dim;
            // Safety: bounds checked by logic above (vec_data len == count * dim)
            let vector_slice = &vec_data[start..end];

            let id = self
                .inner
                .insert(vector_slice, &mut self.storage)
                .map_err(EdgeVecError::from)?;

            if id.0 > u64::from(u32::MAX) {
                return Err(
                    EdgeVecError::Validation("Vector ID overflowed u32".to_string()).into(),
                );
            }
            ids.push(id.0 as u32);
        }

        Ok(Uint32Array::from(&ids[..]))
    }

    /// Inserts multiple vectors using the new batch API (W12.3).
    ///
    /// This method follows the API design from `WASM_BATCH_API.md`:
    /// - Input: Array of Float32Array (each array is one vector)
    /// - Output: BatchInsertResult with inserted count, total, and IDs
    /// - Error codes: EMPTY_BATCH, DIMENSION_MISMATCH, DUPLICATE_ID, etc.
    ///
    /// # Arguments
    ///
    /// * `vectors` - JS Array of Float32Array vectors to insert (1 to 100,000)
    /// * `config` - Optional BatchInsertConfig (default: validateDimensions = true)
    ///
    /// # Returns
    ///
    /// `BatchInsertResult` containing:
    /// - `inserted`: Number of vectors successfully inserted
    /// - `total`: Total vectors attempted (input array length)
    /// - `ids`: Array of IDs for inserted vectors
    ///
    /// # Performance Note
    ///
    /// Batch insert optimizes **JavaScript↔WASM boundary overhead**, not HNSW graph
    /// construction. At smaller batch sizes (100-1K vectors), expect 1.2-1.5x speedup
    /// vs sequential insertion due to reduced FFI calls. At larger scales (5K+), both
    /// methods converge as HNSW graph construction becomes the dominant cost.
    ///
    /// The batch API still provides value at all scales through:
    /// - Simpler API (single call vs loop)
    /// - Atomic operation semantics
    /// - Progress callback support (via `insertBatchWithProgress`)
    ///
    /// # Errors
    ///
    /// Returns a JS error object with `code` property:
    /// - `EMPTY_BATCH`: Input array is empty
    /// - `DIMENSION_MISMATCH`: Vector dimensions don't match index
    /// - `DUPLICATE_ID`: Vector ID already exists
    /// - `INVALID_VECTOR`: Vector contains NaN or Infinity
    /// - `CAPACITY_EXCEEDED`: Batch exceeds max capacity
    /// - `INTERNAL_ERROR`: Internal HNSW error
    #[wasm_bindgen(js_name = insertBatch)]
    pub fn insert_batch_v2(
        &mut self,
        vectors: Array,
        config: Option<batch::BatchInsertConfig>,
    ) -> Result<batch::BatchInsertResult, JsValue> {
        batch::insert_batch_impl(self, vectors, config)
    }

    /// Batch insert with progress callback (W14.1).
    ///
    /// Inserts multiple vectors while reporting progress to a JavaScript callback.
    /// The callback is invoked at the **start (0%)** and **end (100%)** of the batch
    /// insertion. Intermediate progress during insertion is not currently reported.
    ///
    /// # Arguments
    ///
    /// * `vectors` - JS Array of Float32Array vectors to insert
    /// * `on_progress` - JS function called with (inserted: number, total: number)
    ///
    /// # Returns
    ///
    /// `BatchInsertResult` containing inserted count, total, and IDs.
    ///
    /// # Performance Note
    ///
    /// See [`Self::insert_batch_v2`] for performance characteristics. Batch insert optimizes
    /// JS↔WASM boundary overhead (1.2-1.5x at small scales), but converges with
    /// sequential insertion at larger scales as HNSW graph construction dominates.
    ///
    /// # Callback Behavior
    ///
    /// - The callback is called exactly **twice**: once with `(0, total)` before
    ///   insertion begins, and once with `(total, total)` after completion.
    /// - **Errors in the callback are intentionally ignored** — the batch insert
    ///   will succeed even if the progress callback throws an exception. This
    ///   ensures that UI errors don't break data operations.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const result = index.insertBatchWithProgress(vectors, (done, total) => {
    ///     console.log(`Progress: ${Math.round(done/total*100)}%`);
    /// });
    /// console.log(`Inserted ${result.inserted} vectors`);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a JS error object with `code` property on failure.
    /// Note: Callback exceptions do NOT cause this function to return an error.
    #[wasm_bindgen(js_name = insertBatchWithProgress)]
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_batch_with_progress(
        &mut self,
        vectors: Array,
        on_progress: Function,
    ) -> Result<batch::BatchInsertResult, JsValue> {
        let this = JsValue::NULL;
        let total = vectors.length();

        // Report initial progress (0%)
        // INTENTIONAL: Callback errors are silently ignored to ensure batch insert
        // succeeds even if the UI callback fails. This is a deliberate design choice.
        let _ = on_progress.call2(&this, &JsValue::from(0u32), &JsValue::from(total));

        // Perform the batch insert using existing implementation
        let config = batch::BatchInsertConfig::new();
        let result = batch::insert_batch_impl(self, vectors, Some(config))?;

        // Report final progress (100%)
        // INTENTIONAL: Same rationale as above — UI failures shouldn't break data ops.
        let _ = on_progress.call2(&this, &JsValue::from(total), &JsValue::from(total));

        Ok(result)
    }

    /// Searches for nearest neighbors.
    ///
    /// # Arguments
    ///
    /// * `query` - The query vector.
    /// * `k` - The number of neighbors to return.
    ///
    /// # Returns
    ///
    /// An array of objects: `[{ id: u32, score: f32 }, ...]`.
    ///
    /// # Errors
    ///
    /// Returns error if dimensions mismatch or vector contains NaNs.
    #[wasm_bindgen]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search(&self, query: Float32Array, k: usize) -> Result<JsValue, JsValue> {
        let len = query.length();
        if len != self.inner.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: self.inner.config.dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let vec = query.to_vec();
        if vec.iter().any(|v| !v.is_finite()) {
            return Err(EdgeVecError::Validation(
                "Query vector contains non-finite values".to_string(),
            )
            .into());
        }

        let results = self
            .inner
            .search(&vec, k, &self.storage)
            .map_err(EdgeVecError::from)?;

        let arr = Array::new_with_length(results.len() as u32);
        for (i, result) in results.iter().enumerate() {
            let obj = Object::new();
            Reflect::set(
                &obj,
                &JsValue::from_str("id"),
                &JsValue::from(result.vector_id.0 as u32),
            )?;
            Reflect::set(
                &obj,
                &JsValue::from_str("score"),
                &JsValue::from(result.distance),
            )?;
            arr.set(i as u32, obj.into());
        }

        Ok(arr.into())
    }

    /// Creates an iterator to save the database in chunks.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Maximum size of each chunk in bytes (default: 10MB).
    ///
    /// # Returns
    ///
    /// A `PersistenceIterator` that yields `Uint8Array` chunks.
    ///
    /// # Safety
    ///
    /// The returned iterator holds a reference to this `EdgeVec` instance.
    /// You MUST ensure `EdgeVec` is not garbage collected or freed while using the iterator.
    #[wasm_bindgen]
    #[must_use]
    pub fn save_stream(&self, chunk_size: Option<usize>) -> PersistenceIterator {
        let size = chunk_size.unwrap_or(10 * 1024 * 1024); // 10MB default
        let writer = (&self.storage, &self.inner);
        let iter = writer.export_chunked(size);

        // SAFETY: We transmute the lifetime to 'static to allow returning the iterator to JS.
        // JS garbage collection manages the lifetime of EdgeVec.
        // It is the user's responsibility to keep EdgeVec alive while iterating.
        // This is a common pattern in wasm-bindgen for iterators.
        let static_iter = unsafe { std::mem::transmute::<ChunkIter<'_>, ChunkIter<'static>>(iter) };

        PersistenceIterator {
            iter: static_iter,
            liveness: self.liveness.clone(),
        }
    }

    /// Saves the database to IndexedDB.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the database file in IndexedDB.
    ///
    /// # Returns
    ///
    /// A Promise that resolves when saving is complete.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or if the backend write fails.
    #[wasm_bindgen]
    pub async fn save(&self, name: String) -> Result<(), JsValue> {
        let bytes = postcard::to_stdvec(self).map_err(|e| {
            EdgeVecError::Persistence(PersistenceError::Corrupted(format!(
                "Serialization failed: {e}"
            )))
        })?;
        IndexedDbBackend::write(&name, &bytes).await
    }

    /// Loads the database from IndexedDB.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the database file in IndexedDB.
    ///
    /// # Returns
    ///
    /// A Promise that resolves to the loaded EdgeVec instance.
    ///
    /// # Errors
    ///
    /// Returns an error if loading fails, deserialization fails, or data is corrupted.
    #[wasm_bindgen]
    pub async fn load(name: String) -> Result<EdgeVec, JsValue> {
        // [m1] Ensure logging is initialized on load as well
        INIT.call_once(|| {
            init_logging();
        });

        let val = IndexedDbBackend::read(&name).await?;
        let bytes = Uint8Array::new(&val).to_vec();

        let mut edge_vec: EdgeVec = postcard::from_bytes(&bytes).map_err(|e| {
            EdgeVecError::Persistence(PersistenceError::Corrupted(format!(
                "Deserialization failed: {e}"
            )))
        })?;

        // Restore liveness (skipped during serialization)
        edge_vec.liveness = Arc::new(AtomicBool::new(true));

        Ok(edge_vec)
    }

    // =========================================================================
    // SOFT DELETE API (v0.3.0 — RFC-001)
    // =========================================================================

    /// Soft delete a vector by marking it as a tombstone.
    ///
    /// The vector remains in the index but is excluded from search results.
    /// Space is reclaimed via `compact()` when tombstone ratio exceeds threshold.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector to delete (returned from `insert`).
    ///
    /// # Returns
    ///
    /// * `true` if the vector was deleted
    /// * `false` if the vector was already deleted (idempotent)
    ///
    /// # Errors
    ///
    /// Returns an error if the vector ID doesn't exist.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const id = index.insert(new Float32Array(128).fill(1.0));
    /// const wasDeleted = index.softDelete(id);
    /// console.log(`Deleted: ${wasDeleted}`); // true
    /// console.log(`Is deleted: ${index.isDeleted(id)}`); // true
    /// ```
    #[wasm_bindgen(js_name = softDelete)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn soft_delete(&mut self, vector_id: u32) -> Result<bool, JsValue> {
        let id = crate::hnsw::VectorId(u64::from(vector_id));
        self.inner
            .soft_delete(id)
            .map_err(|e| JsValue::from_str(&format!("soft_delete failed: {e}")))
    }

    /// Check if a vector is deleted (tombstoned).
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector to check.
    ///
    /// # Returns
    ///
    /// * `true` if the vector is deleted
    /// * `false` if the vector is live
    ///
    /// # Errors
    ///
    /// Returns an error if the vector ID doesn't exist.
    #[wasm_bindgen(js_name = isDeleted)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn is_deleted(&self, vector_id: u32) -> Result<bool, JsValue> {
        let id = crate::hnsw::VectorId(u64::from(vector_id));
        self.inner
            .is_deleted(id)
            .map_err(|e| JsValue::from_str(&format!("is_deleted failed: {e}")))
    }

    /// Get the count of deleted (tombstoned) vectors.
    ///
    /// # Returns
    ///
    /// The number of vectors that have been soft-deleted but not yet compacted.
    #[wasm_bindgen(js_name = deletedCount)]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn deleted_count(&self) -> u32 {
        self.inner.deleted_count() as u32
    }

    /// Get the count of live (non-deleted) vectors.
    ///
    /// # Returns
    ///
    /// The number of vectors that are currently searchable.
    #[wasm_bindgen(js_name = liveCount)]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn live_count(&self) -> u32 {
        self.inner.live_count() as u32
    }

    /// Get the ratio of deleted to total vectors.
    ///
    /// # Returns
    ///
    /// A value between 0.0 and 1.0 representing the tombstone ratio.
    /// 0.0 means no deletions, 1.0 means all vectors deleted.
    #[wasm_bindgen(js_name = tombstoneRatio)]
    #[must_use]
    pub fn tombstone_ratio(&self) -> f64 {
        self.inner.tombstone_ratio()
    }

    /// Check if compaction is recommended.
    ///
    /// Returns `true` when `tombstoneRatio()` exceeds the compaction threshold
    /// (default: 30%). Use `compact()` to reclaim space from deleted vectors.
    ///
    /// # Returns
    ///
    /// * `true` if compaction is recommended
    /// * `false` if tombstone ratio is below threshold
    #[wasm_bindgen(js_name = needsCompaction)]
    #[must_use]
    pub fn needs_compaction(&self) -> bool {
        self.inner.needs_compaction()
    }

    /// Get the current compaction threshold.
    ///
    /// # Returns
    ///
    /// The threshold ratio (0.0 to 1.0) above which `needsCompaction()` returns true.
    /// Default is 0.3 (30%).
    #[wasm_bindgen(js_name = compactionThreshold)]
    #[must_use]
    pub fn compaction_threshold(&self) -> f64 {
        self.inner.compaction_threshold()
    }

    /// Set the compaction threshold.
    ///
    /// # Arguments
    ///
    /// * `ratio` - The new threshold (clamped to 0.01 - 0.99).
    #[wasm_bindgen(js_name = setCompactionThreshold)]
    pub fn set_compaction_threshold(&mut self, ratio: f64) {
        self.inner.set_compaction_threshold(ratio);
    }

    /// Get a warning message if compaction is recommended.
    ///
    /// # Returns
    ///
    /// * A warning string if `needsCompaction()` is true
    /// * `null` if compaction is not needed
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const warning = index.compactionWarning();
    /// if (warning) {
    ///     console.warn(warning);
    ///     index.compact();
    /// }
    /// ```
    #[wasm_bindgen(js_name = compactionWarning)]
    #[must_use]
    pub fn compaction_warning(&self) -> Option<String> {
        self.inner.compaction_warning()
    }

    /// Compact the index by rebuilding without tombstones.
    ///
    /// This operation:
    /// 1. Creates a new index with only live vectors
    /// 2. Re-inserts vectors preserving IDs
    /// 3. Replaces the current index
    ///
    /// **WARNING:** This is a blocking operation. For indices with >10k vectors,
    /// consider running during idle time or warning the user about potential delays.
    ///
    /// # Returns
    ///
    /// A `CompactionResult` object containing:
    /// * `tombstonesRemoved` - Number of deleted vectors removed
    /// * `newSize` - Size of the index after compaction
    /// * `durationMs` - Time taken in milliseconds
    ///
    /// # Errors
    ///
    /// Returns an error if compaction fails (e.g., memory allocation error).
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// if (index.needsCompaction()) {
    ///     const result = index.compact();
    ///     console.log(`Removed ${result.tombstonesRemoved} tombstones`);
    ///     console.log(`New size: ${result.newSize}`);
    ///     console.log(`Took ${result.durationMs}ms`);
    /// }
    /// ```
    #[wasm_bindgen]
    #[allow(clippy::cast_possible_truncation)]
    pub fn compact(&mut self) -> Result<WasmCompactionResult, JsValue> {
        let (new_index, new_storage, result) = self
            .inner
            .compact(&self.storage)
            .map_err(|e| JsValue::from_str(&format!("compact failed: {e}")))?;

        // Replace internal state with compacted versions
        self.inner = new_index;
        self.storage = new_storage;

        Ok(WasmCompactionResult {
            tombstones_removed: result.tombstones_removed as u32,
            new_size: result.new_size as u32,
            duration_ms: result.duration_ms as u32,
        })
    }

    // =========================================================================
    // BATCH DELETE API (W18.5 — RFC-001)
    // =========================================================================

    /// Soft-delete multiple vectors using BigUint64Array (modern browsers).
    ///
    /// Efficiently deletes multiple vectors in a single operation. More efficient
    /// than calling `softDelete()` N times due to reduced FFI overhead and
    /// deduplication of input IDs.
    ///
    /// **Browser Compatibility:** Requires BigUint64Array support (Chrome 67+,
    /// Firefox 68+, Safari 15+). For Safari 14 compatibility, use
    /// `softDeleteBatchCompat()` instead.
    ///
    /// # Arguments
    ///
    /// * `ids` - A Uint32Array of vector IDs to delete
    ///
    /// # Returns
    ///
    /// A `WasmBatchDeleteResult` object containing:
    /// * `deleted` - Number of vectors successfully deleted
    /// * `alreadyDeleted` - Number of vectors that were already deleted
    /// * `invalidIds` - Number of IDs not found in the index
    /// * `total` - Total IDs in input (including duplicates)
    /// * `uniqueCount` - Number of unique IDs after deduplication
    ///
    /// # Behavior
    ///
    /// * **Deduplication:** Duplicate IDs in input are processed only once
    /// * **Idempotent:** Re-deleting an already-deleted vector returns `alreadyDeleted`
    /// * **Atomic:** Two-phase validation ensures all-or-nothing semantics
    ///
    /// # Errors
    ///
    /// Returns an error if the batch size exceeds the maximum (10M IDs).
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const ids = new Uint32Array([1, 3, 5, 7, 9, 11]);
    /// const result = index.softDeleteBatch(ids);
    ///
    /// console.log(`Deleted: ${result.deleted}`);
    /// console.log(`Already deleted: ${result.alreadyDeleted}`);
    /// console.log(`Invalid IDs: ${result.invalidIds}`);
    /// console.log(`All valid: ${result.allValid()}`);
    /// ```
    #[wasm_bindgen(js_name = softDeleteBatch)]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn soft_delete_batch(
        &mut self,
        ids: js_sys::Uint32Array,
    ) -> Result<WasmBatchDeleteResult, JsValue> {
        // Convert Uint32Array to Vec<VectorId>
        let id_vec: Vec<u32> = ids.to_vec();
        let vec_ids: Vec<crate::hnsw::VectorId> = id_vec
            .iter()
            .map(|&id| crate::hnsw::VectorId(u64::from(id)))
            .collect();

        // Call core batch delete
        let result = self.inner.soft_delete_batch(&vec_ids);

        Ok(WasmBatchDeleteResult {
            deleted: result.deleted as u32,
            already_deleted: result.already_deleted as u32,
            invalid_ids: result.invalid_ids as u32,
            total: result.total as u32,
            unique_count: result.unique_count as u32,
        })
    }

    /// Soft-delete multiple vectors using number array (Safari 14 compatible).
    ///
    /// This method provides Safari 14 compatibility by accepting a regular JavaScript
    /// Array of numbers instead of BigUint64Array. IDs must be less than 2^53
    /// (Number.MAX_SAFE_INTEGER) to avoid precision loss.
    ///
    /// **Note:** For modern browsers, prefer `softDeleteBatch()` which uses typed arrays.
    ///
    /// # Arguments
    ///
    /// * `ids` - A JavaScript Array or Float64Array of vector IDs
    ///
    /// # Returns
    ///
    /// Same as `softDeleteBatch()` - see that method for details.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch size exceeds the maximum (10M IDs) or if
    /// any ID exceeds Number.MAX_SAFE_INTEGER.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// // Safari 14 compatible
    /// const ids = [1, 3, 5, 7, 9, 11];
    /// const result = index.softDeleteBatchCompat(ids);
    /// console.log(`Deleted: ${result.deleted}`);
    /// ```
    #[wasm_bindgen(js_name = softDeleteBatchCompat)]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn soft_delete_batch_compat(
        &mut self,
        ids: js_sys::Float64Array,
    ) -> Result<WasmBatchDeleteResult, JsValue> {
        // Convert Float64Array to Vec<VectorId>
        // Safe for IDs < 2^53 (Number.MAX_SAFE_INTEGER)
        let id_vec: Vec<f64> = ids.to_vec();
        let vec_ids: Vec<crate::hnsw::VectorId> = id_vec
            .iter()
            .map(|&id| crate::hnsw::VectorId(id as u64))
            .collect();

        // Call core batch delete
        let result = self.inner.soft_delete_batch(&vec_ids);

        Ok(WasmBatchDeleteResult {
            deleted: result.deleted as u32,
            already_deleted: result.already_deleted as u32,
            invalid_ids: result.invalid_ids as u32,
            total: result.total as u32,
            unique_count: result.unique_count as u32,
        })
    }

    // =========================================================================
    // METADATA API (v0.5.0 — Week 21)
    // =========================================================================

    /// Sets metadata for a vector (upsert operation).
    ///
    /// If the key already exists, its value is overwritten. If the key is new,
    /// it is added (subject to the 64-key-per-vector limit).
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector to attach metadata to
    /// * `key` - The metadata key (alphanumeric + underscore, max 256 chars)
    /// * `value` - The metadata value (created via JsMetadataValue.fromX methods)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Key is empty or contains invalid characters
    /// - Key exceeds 256 characters
    /// - Value validation fails (e.g., NaN float, string too long)
    /// - Vector already has 64 keys and this is a new key
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const id = index.insert(vector);
    /// index.setMetadata(id, 'title', JsMetadataValue.fromString('My Document'));
    /// index.setMetadata(id, 'page_count', JsMetadataValue.fromInteger(42));
    /// index.setMetadata(id, 'score', JsMetadataValue.fromFloat(0.95));
    /// index.setMetadata(id, 'verified', JsMetadataValue.fromBoolean(true));
    /// ```
    #[wasm_bindgen(js_name = "setMetadata")]
    pub fn set_metadata(
        &mut self,
        vector_id: u32,
        key: &str,
        value: &metadata::JsMetadataValue,
    ) -> Result<(), JsError> {
        self.metadata
            .insert(vector_id, key, value.inner.clone())
            .map_err(metadata::metadata_error_to_js)
    }

    /// Gets metadata for a vector.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector
    /// * `key` - The metadata key to retrieve
    ///
    /// # Returns
    ///
    /// The metadata value, or `undefined` if the key or vector doesn't exist.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const title = index.getMetadata(id, 'title');
    /// if (title) {
    ///     console.log('Title:', title.asString());
    ///     console.log('Type:', title.getType());
    /// }
    /// ```
    #[wasm_bindgen(js_name = "getMetadata")]
    #[must_use]
    pub fn get_metadata(&self, vector_id: u32, key: &str) -> Option<metadata::JsMetadataValue> {
        metadata::metadata_value_to_js(self.metadata.get(vector_id, key))
    }

    /// Gets all metadata for a vector as a JavaScript object.
    ///
    /// Returns a plain JavaScript object where keys are metadata keys and
    /// values are JavaScript-native types (string, number, boolean, string[]).
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector
    ///
    /// # Returns
    ///
    /// A JavaScript object mapping keys to values, or `undefined` if the vector
    /// has no metadata.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const metadata = index.getAllMetadata(id);
    /// if (metadata) {
    ///     console.log(metadata.title);     // 'My Document'
    ///     console.log(metadata.page_count); // 42
    ///     console.log(Object.keys(metadata)); // ['title', 'page_count', ...]
    /// }
    /// ```
    #[wasm_bindgen(js_name = "getAllMetadata")]
    #[must_use]
    pub fn get_all_metadata(&self, vector_id: u32) -> JsValue {
        metadata::metadata_to_js_object(&self.metadata, vector_id)
    }

    /// Deletes a metadata key for a vector.
    ///
    /// This operation is idempotent - deleting a non-existent key is not an error.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector
    /// * `key` - The metadata key to delete
    ///
    /// # Returns
    ///
    /// `true` if the key existed and was deleted, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the key is invalid (empty or contains invalid characters).
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const wasDeleted = index.deleteMetadata(id, 'title');
    /// console.log(wasDeleted); // true if key existed
    /// ```
    #[wasm_bindgen(js_name = "deleteMetadata")]
    pub fn delete_metadata(&mut self, vector_id: u32, key: &str) -> Result<bool, JsError> {
        self.metadata
            .delete(vector_id, key)
            .map_err(metadata::metadata_error_to_js)
    }

    /// Deletes all metadata for a vector.
    ///
    /// This operation is idempotent - deleting metadata for a vector without
    /// metadata is not an error.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector
    ///
    /// # Returns
    ///
    /// `true` if the vector had metadata that was deleted, `false` otherwise.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const hadMetadata = index.deleteAllMetadata(id);
    /// console.log(hadMetadata); // true if vector had any metadata
    /// ```
    #[wasm_bindgen(js_name = "deleteAllMetadata")]
    pub fn delete_all_metadata(&mut self, vector_id: u32) -> bool {
        self.metadata.delete_all(vector_id)
    }

    /// Checks if a metadata key exists for a vector.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector
    /// * `key` - The metadata key to check
    ///
    /// # Returns
    ///
    /// `true` if the key exists, `false` otherwise.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// if (index.hasMetadata(id, 'title')) {
    ///     console.log('Vector has title metadata');
    /// }
    /// ```
    #[wasm_bindgen(js_name = "hasMetadata")]
    #[must_use]
    pub fn has_metadata(&self, vector_id: u32, key: &str) -> bool {
        self.metadata.has_key(vector_id, key)
    }

    /// Returns the number of metadata keys for a vector.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector
    ///
    /// # Returns
    ///
    /// The number of metadata keys, or 0 if the vector has no metadata.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const count = index.metadataKeyCount(id);
    /// console.log(`Vector has ${count} metadata keys`);
    /// ```
    #[wasm_bindgen(js_name = "metadataKeyCount")]
    #[must_use]
    pub fn metadata_key_count(&self, vector_id: u32) -> usize {
        self.metadata.key_count(vector_id)
    }

    /// Returns the total number of vectors with metadata.
    ///
    /// # Returns
    ///
    /// The count of vectors that have at least one metadata key.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const count = index.metadataVectorCount();
    /// console.log(`${count} vectors have metadata`);
    /// ```
    #[wasm_bindgen(js_name = "metadataVectorCount")]
    #[must_use]
    pub fn metadata_vector_count(&self) -> usize {
        self.metadata.vector_count()
    }

    /// Returns the total number of metadata key-value pairs across all vectors.
    ///
    /// # Returns
    ///
    /// The total count of metadata entries.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const total = index.totalMetadataCount();
    /// console.log(`${total} total metadata entries`);
    /// ```
    #[wasm_bindgen(js_name = "totalMetadataCount")]
    #[must_use]
    pub fn total_metadata_count(&self) -> usize {
        self.metadata.total_key_count()
    }
}

/// Result of a compaction operation (v0.3.0).
///
/// Returned by `EdgeVec.compact()` to provide metrics about the operation.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmCompactionResult {
    /// Number of tombstones (deleted vectors) removed during compaction.
    #[wasm_bindgen(readonly)]
    pub tombstones_removed: u32,

    /// New index size after compaction (live vectors only).
    #[wasm_bindgen(readonly)]
    pub new_size: u32,

    /// Time taken for the compaction operation in milliseconds.
    #[wasm_bindgen(readonly)]
    pub duration_ms: u32,
}

/// Result of a batch delete operation (W18.4/W18.5).
///
/// Returned by `EdgeVec.softDeleteBatch()` and `EdgeVec.softDeleteBatchCompat()`
/// to provide detailed metrics about the batch deletion.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmBatchDeleteResult {
    deleted: u32,
    already_deleted: u32,
    invalid_ids: u32,
    total: u32,
    unique_count: u32,
}

#[wasm_bindgen]
impl WasmBatchDeleteResult {
    /// Number of vectors successfully deleted in this operation.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn deleted(&self) -> u32 {
        self.deleted
    }

    /// Number of vectors that were already deleted (tombstoned).
    #[wasm_bindgen(getter, js_name = "alreadyDeleted")]
    #[must_use]
    pub fn already_deleted(&self) -> u32 {
        self.already_deleted
    }

    /// Number of invalid IDs (not found in the index).
    #[wasm_bindgen(getter, js_name = "invalidIds")]
    #[must_use]
    pub fn invalid_ids(&self) -> u32 {
        self.invalid_ids
    }

    /// Total number of vector IDs provided in the input (including duplicates).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Number of unique vector IDs after deduplication.
    #[wasm_bindgen(getter, js_name = "uniqueCount")]
    #[must_use]
    pub fn unique_count(&self) -> u32 {
        self.unique_count
    }

    /// Check if all operations succeeded (no invalid IDs).
    ///
    /// Returns `true` if every ID was valid (either deleted or already deleted).
    #[wasm_bindgen(js_name = "allValid")]
    #[must_use]
    pub fn all_valid(&self) -> bool {
        self.invalid_ids == 0
    }

    /// Check if any deletions occurred in this operation.
    ///
    /// Returns `true` if at least one vector was newly deleted.
    #[wasm_bindgen(js_name = "anyDeleted")]
    #[must_use]
    pub fn any_deleted(&self) -> bool {
        self.deleted > 0
    }
}

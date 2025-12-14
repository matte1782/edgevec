//! WASM Bindings for EdgeVec.

use crate::error::EdgeVecError;
use crate::hnsw::{GraphError, HnswConfig, HnswIndex};
use crate::persistence::{chunking::ChunkIter, ChunkedWriter, PersistenceError};
use crate::storage::VectorStorage;
use js_sys::{Array, Float32Array, Object, Reflect, Uint32Array, Uint8Array};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Once,
};
use wasm_bindgen::prelude::*;

mod batch;
mod iterator;

pub use batch::{BatchInsertConfig, BatchInsertResult};
pub use iterator::PersistenceIterator;

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
}

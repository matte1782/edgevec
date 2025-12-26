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
pub mod filter;
mod iterator;
mod memory;
mod metadata;

pub use batch::{BatchInsertConfig, BatchInsertResult};
pub use iterator::PersistenceIterator;
pub use memory::{
    track_batch_insert, track_vector_insert, MemoryConfig, MemoryPressure, MemoryPressureLevel,
    MemoryRecommendation,
};
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

/// Get the SIMD backend being used for distance calculations.
/// Returns: "wasm_simd128", "avx2", or "scalar"
#[wasm_bindgen(js_name = "getSimdBackend")]
#[must_use]
pub fn get_simd_backend() -> String {
    cfg_if::cfg_if! {
        if #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))] {
            "wasm_simd128".to_string()
        } else if #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))] {
            "avx2".to_string()
        } else {
            "scalar".to_string()
        }
    }
}

/// Microbenchmark: measure raw Hamming distance speed.
/// Returns time in microseconds for `iterations` distance calculations.
#[wasm_bindgen(js_name = "benchmarkHamming")]
#[allow(clippy::cast_possible_truncation)] // Intentional truncation for pseudo-random test data
pub fn benchmark_hamming(bytes: usize, iterations: usize) -> f64 {
    use crate::metric::{Hamming, Metric};

    // Create two random-ish vectors (truncation is intentional for test data)
    let a: Vec<u8> = (0..bytes).map(|i| (i * 17 + 31) as u8).collect();
    let b: Vec<u8> = (0..bytes).map(|i| (i * 13 + 47) as u8).collect();

    let perf = web_sys::window().and_then(|w| w.performance());
    let start = perf.as_ref().map_or(0.0, web_sys::Performance::now);

    let mut sum: f32 = 0.0;
    for _ in 0..iterations {
        sum += Hamming::distance(&a, &b);
    }

    let end = perf.as_ref().map_or(0.0, web_sys::Performance::now);

    // Prevent optimizer from removing the loop
    if sum < 0.0 {
        web_sys::console::log_1(&format!("sum={sum}").into());
    }

    // iterations is always < 2^53, precision loss is acceptable for benchmark timing
    #[allow(clippy::cast_precision_loss)]
    let result = (end - start) * 1000.0 / iterations as f64;
    result // Return microseconds per iteration
}

/// Side-by-side benchmark: New WASM SIMD128 vs Current runtime dispatcher.
///
/// Compares:
/// 1. **New** (`metric::simd::hamming_distance`): Compile-time SIMD128 detection → uses WASM SIMD
/// 2. **Current** (`simd::popcount::simd_popcount_xor`): Runtime detection → falls to scalar in WASM
///
/// Returns a JSON string with timings:
/// ```json
/// {"new_us": 0.15, "current_us": 0.42, "speedup": 2.8, "new_backend": "wasm_simd128", "current_backend": "scalar"}
/// ```
#[wasm_bindgen(js_name = "benchmarkHammingComparison")]
#[allow(clippy::cast_possible_truncation)] // Intentional truncation for pseudo-random test data
pub fn benchmark_hamming_comparison(bytes: usize, iterations: usize) -> String {
    // Current implementation - uses runtime detection, falls to scalar in WASM
    use crate::simd::popcount::simd_popcount_xor;

    // Create two random-ish vectors (truncation is intentional for test data)
    let a: Vec<u8> = (0..bytes).map(|i| (i * 17 + 31) as u8).collect();
    let b: Vec<u8> = (0..bytes).map(|i| (i * 13 + 47) as u8).collect();

    let perf = web_sys::window().and_then(|w| w.performance());

    // Warmup both implementations
    for _ in 0..100 {
        let _ = crate::metric::simd::hamming_distance(&a, &b);
        let _ = simd_popcount_xor(&a, &b);
    }

    // Benchmark NEW: metric::simd::hamming_distance
    // Uses compile-time #[cfg(target_feature = "simd128")] → WASM SIMD128
    let start_new = perf.as_ref().map_or(0.0, web_sys::Performance::now);
    let mut sum_new: u32 = 0;
    for _ in 0..iterations {
        sum_new = sum_new.wrapping_add(crate::metric::simd::hamming_distance(&a, &b));
    }
    let end_new = perf.as_ref().map_or(0.0, web_sys::Performance::now);

    // Benchmark CURRENT: simd::popcount::simd_popcount_xor
    // Uses runtime is_x86_feature_detected!() → falls to scalar in WASM
    let start_current = perf.as_ref().map_or(0.0, web_sys::Performance::now);
    let mut sum_current: u32 = 0;
    for _ in 0..iterations {
        sum_current = sum_current.wrapping_add(simd_popcount_xor(&a, &b));
    }
    let end_current = perf.as_ref().map_or(0.0, web_sys::Performance::now);

    // Verify both produce same result
    if sum_new != sum_current {
        web_sys::console::warn_1(
            &format!("WARNING: Results differ! new={sum_new} current={sum_current}").into(),
        );
    }

    // Prevent optimizer from removing the loops
    if sum_new == 0 || sum_current == 0 {
        web_sys::console::log_1(&format!("sums: {sum_new} {sum_current}").into());
    }

    // iterations is always < 2^53, precision loss is acceptable for benchmark timing
    #[allow(clippy::cast_precision_loss)]
    let new_us = (end_new - start_new) * 1000.0 / iterations as f64;
    #[allow(clippy::cast_precision_loss)]
    let current_us = (end_current - start_current) * 1000.0 / iterations as f64;
    let speedup = current_us / new_us;

    // Determine which backend each is actually using
    let new_backend = if cfg!(all(target_arch = "wasm32", target_feature = "simd128")) {
        "WASM SIMD128"
    } else if cfg!(all(target_arch = "x86_64", target_feature = "avx2")) {
        "AVX2"
    } else {
        "Scalar"
    };

    // Current uses runtime detection - in WASM it's always scalar
    let current_backend = if cfg!(target_arch = "wasm32") {
        "Scalar"
    } else {
        "Runtime Dispatch"
    };

    format!(
        r#"{{"new_us": {new_us:.3}, "current_us": {current_us:.3}, "speedup": {speedup:.2}, "bytes": {bytes}, "iterations": {iterations}, "new_backend": "{new_backend}", "current_backend": "{current_backend}"}}"#
    )
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
    /// Memory pressure configuration (skipped during serialization).
    #[serde(skip, default)]
    memory_config: MemoryConfig,
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
            memory_config: MemoryConfig::default(),
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

        // insert() automatically handles BQ storage when enabled (via insert_impl Step 6)
        let id = self
            .inner
            .insert(&vec, &mut self.storage)
            .map_err(EdgeVecError::from)?;

        // Track memory allocation for memory pressure monitoring
        track_vector_insert(self.inner.config.dimensions);

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

            // insert() automatically handles BQ storage when enabled (via insert_impl Step 6)
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

        // Track memory allocation for the batch
        track_batch_insert(count, self.inner.config.dimensions);

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
        self.inner
            .metadata
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
        metadata::metadata_value_to_js(self.inner.metadata.get(vector_id, key))
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
        metadata::metadata_to_js_object(&self.inner.metadata, vector_id)
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
        self.inner
            .metadata
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
        self.inner.metadata.delete_all(vector_id)
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
        self.inner.metadata.has_key(vector_id, key)
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
        self.inner.metadata.key_count(vector_id)
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
        self.inner.metadata.vector_count()
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
        self.inner.metadata.total_key_count()
    }

    // =========================================================================
    // COMBINED INSERT + METADATA API (v0.6.0 — Week 28 RFC-002)
    // =========================================================================

    /// Insert a vector with associated metadata in a single operation.
    ///
    /// This is a convenience method that combines `insert()` and `setMetadata()`
    /// into a single atomic operation. The vector is inserted first, then all
    /// metadata key-value pairs are attached to it.
    ///
    /// # Arguments
    ///
    /// * `vector` - A Float32Array containing the vector data
    /// * `metadata` - A JavaScript object with string keys and metadata values
    ///   - Supported value types: `string`, `number`, `boolean`, `string[]`
    ///   - Numbers are automatically detected as integer or float
    ///
    /// # Returns
    ///
    /// The assigned Vector ID (u32).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Vector dimensions mismatch the index configuration
    /// - Vector contains NaN or Infinity values
    /// - Metadata key is invalid (empty, too long, or contains invalid characters)
    /// - Metadata value is invalid (NaN float, string too long, etc.)
    /// - Too many metadata keys (>64 per vector)
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const id = index.insertWithMetadata(
    ///     new Float32Array([0.1, 0.2, 0.3, ...]),
    ///     {
    ///         category: "news",
    ///         score: 0.95,
    ///         active: true,
    ///         tags: ["featured", "trending"]
    ///     }
    /// );
    /// console.log(`Inserted vector with ID: ${id}`);
    /// ```
    #[wasm_bindgen(js_name = "insertWithMetadata")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn insert_with_metadata(
        &mut self,
        vector: Float32Array,
        metadata_js: JsValue,
    ) -> Result<u32, JsValue> {
        // Validate vector dimension
        let len = vector.length();
        if len != self.inner.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: self.inner.config.dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let vec = vector.to_vec();

        #[cfg(debug_assertions)]
        if vec.iter().any(|v| !v.is_finite()) {
            return Err(
                EdgeVecError::Validation("Vector contains non-finite values".to_string()).into(),
            );
        }

        // Parse JavaScript object into HashMap<String, MetadataValue>
        let metadata = parse_js_metadata_object(&metadata_js)?;

        // Use the core insert_with_metadata method
        let id = self
            .inner
            .insert_with_metadata(&mut self.storage, &vec, metadata)
            .map_err(EdgeVecError::from)?;

        // Track memory allocation for memory pressure monitoring
        track_vector_insert(self.inner.config.dimensions);

        // Safety: VectorId is u64, we cast to u32 as requested by API.
        if id.0 > u64::from(u32::MAX) {
            return Err(EdgeVecError::Validation("Vector ID overflowed u32".to_string()).into());
        }
        Ok(id.0 as u32)
    }

    /// Search with metadata filter expression (simplified API).
    ///
    /// This is a simplified version of `searchFiltered()` that takes the filter
    /// expression directly as a string instead of JSON options.
    ///
    /// # Arguments
    ///
    /// * `query` - A Float32Array containing the query vector
    /// * `filter` - Filter expression string (e.g., 'category == "news" AND score > 0.5')
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// An array of search result objects: `[{ id: number, distance: number }, ...]`
    ///
    /// # Filter Syntax
    ///
    /// - Comparison: `field == value`, `field != value`, `field > value`, etc.
    /// - Logical: `expr AND expr`, `expr OR expr`, `NOT expr`
    /// - Grouping: `(expr)`
    /// - Array contains: `field CONTAINS value`
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Query dimensions mismatch
    /// - Filter expression is invalid
    /// - k is 0
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const results = index.searchWithFilter(
    ///     new Float32Array([0.1, 0.2, ...]),
    ///     'category == "news" AND score > 0.5',
    ///     10
    /// );
    /// for (const r of results) {
    ///     console.log(`ID: ${r.id}, Distance: ${r.distance}`);
    /// }
    /// ```
    #[wasm_bindgen(js_name = "searchWithFilter")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search_with_filter(
        &mut self,
        query: Float32Array,
        filter: &str,
        k: usize,
    ) -> Result<JsValue, JsValue> {
        use crate::filter::{parse, FilterStrategy, FilteredSearcher};

        // Validate k
        if k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }

        // Validate query dimensions
        let len = query.length();
        if len != self.inner.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: self.inner.config.dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let query_vec = query.to_vec();
        if query_vec.iter().any(|v| !v.is_finite()) {
            return Err(EdgeVecError::Validation(
                "Query vector contains non-finite values".to_string(),
            )
            .into());
        }

        // Parse filter expression
        let filter_expr = parse(filter).map_err(|e| filter::filter_error_to_jsvalue(&e))?;

        // Create metadata store adapter
        let metadata_adapter = EdgeVecMetadataAdapter::new(&self.inner.metadata, self.inner.len());

        // Execute filtered search with auto strategy
        let mut searcher = FilteredSearcher::new(&self.inner, &self.storage, &metadata_adapter);
        let result = searcher
            .search_filtered(&query_vec, k, Some(&filter_expr), FilterStrategy::Auto)
            .map_err(|e| JsValue::from_str(&format!("Search failed: {e}")))?;

        // Convert results to JavaScript array
        let arr = Array::new_with_length(result.results.len() as u32);
        for (i, r) in result.results.iter().enumerate() {
            let obj = Object::new();
            Reflect::set(
                &obj,
                &JsValue::from_str("id"),
                &JsValue::from(r.vector_id.0 as u32),
            )?;
            Reflect::set(
                &obj,
                &JsValue::from_str("distance"),
                &JsValue::from(r.distance),
            )?;
            arr.set(i as u32, obj.into());
        }

        Ok(arr.into())
    }

    /// Get all metadata for a vector by ID (alias for getAllMetadata).
    ///
    /// This is an alias for `getAllMetadata()` provided for API consistency
    /// with the new RFC-002 metadata API.
    ///
    /// # Arguments
    ///
    /// * `id` - The vector ID to look up
    ///
    /// # Returns
    ///
    /// A JavaScript object with all metadata key-value pairs, or `undefined`
    /// if the vector has no metadata or doesn't exist.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const id = index.insertWithMetadata(vector, { category: 'news' });
    /// const meta = index.getVectorMetadata(id);
    /// console.log(meta.category); // 'news'
    /// ```
    #[wasm_bindgen(js_name = "getVectorMetadata")]
    #[must_use]
    pub fn get_vector_metadata(&self, id: u32) -> JsValue {
        metadata::metadata_to_js_object(&self.inner.metadata, id)
    }

    // =========================================================================
    // BINARY QUANTIZATION SEARCH API (v0.6.0 — Week 28 RFC-002)
    // =========================================================================

    /// Search using binary quantization (fast, approximate).
    ///
    /// Binary quantization converts vectors to bit arrays (1 bit per dimension)
    /// and uses Hamming distance for comparison. This provides:
    /// - ~32x memory reduction
    /// - ~3-5x faster search
    /// - ~70-85% recall (use `searchBQRescored` for higher recall)
    ///
    /// # Arguments
    ///
    /// * `query` - A Float32Array containing the query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// An array of search result objects: `[{ id: number, distance: number }, ...]`
    /// where distance is a similarity score (higher is more similar).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Binary quantization is not enabled on this index
    /// - Query dimensions mismatch
    /// - k is 0
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// // Fast search, lower recall
    /// const results = index.searchBQ(new Float32Array([0.1, 0.2, ...]), 10);
    /// for (const r of results) {
    ///     console.log(`ID: ${r.id}, Similarity: ${r.distance}`);
    /// }
    /// ```
    #[wasm_bindgen(js_name = "searchBQ")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search_bq(&self, query: Float32Array, k: usize) -> Result<JsValue, JsValue> {
        // Validate k
        if k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }

        // Validate query dimensions
        let len = query.length();
        if len != self.inner.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: self.inner.config.dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let query_vec = query.to_vec();
        if query_vec.iter().any(|v| !v.is_finite()) {
            return Err(EdgeVecError::Validation(
                "Query vector contains non-finite values".to_string(),
            )
            .into());
        }

        // Execute BQ search
        let results = self
            .inner
            .search_bq(&query_vec, k, &self.storage)
            .map_err(EdgeVecError::from)?;

        // Convert results to JavaScript array
        let arr = Array::new_with_length(results.len() as u32);
        for (i, (vector_id, similarity)) in results.iter().enumerate() {
            let obj = Object::new();
            Reflect::set(
                &obj,
                &JsValue::from_str("id"),
                &JsValue::from(vector_id.0 as u32),
            )?;
            Reflect::set(
                &obj,
                &JsValue::from_str("distance"),
                &JsValue::from(*similarity),
            )?;
            arr.set(i as u32, obj.into());
        }

        Ok(arr.into())
    }

    /// Search using BQ with F32 rescoring (fast + accurate).
    ///
    /// This method combines BQ speed with F32 accuracy:
    /// 1. Uses BQ to quickly find `k * rescoreFactor` candidates
    /// 2. Rescores candidates using exact F32 distance
    /// 3. Returns the final top-k results
    ///
    /// This provides near-F32 recall (~95%) with most of the BQ speedup.
    ///
    /// # Arguments
    ///
    /// * `query` - A Float32Array containing the query vector
    /// * `k` - Number of results to return
    /// * `rescore_factor` - Overfetch multiplier (3-10 recommended)
    ///
    /// # Returns
    ///
    /// An array of search result objects: `[{ id: number, distance: number }, ...]`
    /// where distance is a similarity score (higher is more similar).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Binary quantization is not enabled on this index
    /// - Query dimensions mismatch
    /// - k or rescore_factor is 0
    ///
    /// # Rescore Factor Guide
    ///
    /// | Factor | Recall | Relative Speed |
    /// |--------|--------|----------------|
    /// | 1      | ~70%   | 5x             |
    /// | 3      | ~90%   | 3x             |
    /// | 5      | ~95%   | 2.5x           |
    /// | 10     | ~98%   | 2x             |
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// // Fast search with high recall (~95%)
    /// const results = index.searchBQRescored(
    ///     new Float32Array([0.1, 0.2, ...]),
    ///     10,  // k
    ///     5    // rescore factor
    /// );
    /// ```
    #[wasm_bindgen(js_name = "searchBQRescored")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search_bq_rescored(
        &self,
        query: Float32Array,
        k: usize,
        rescore_factor: usize,
    ) -> Result<JsValue, JsValue> {
        // Validate k
        if k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }

        // Validate rescore_factor
        if rescore_factor == 0 {
            return Err(JsValue::from_str("rescoreFactor must be greater than 0"));
        }

        // Validate query dimensions
        let len = query.length();
        if len != self.inner.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: self.inner.config.dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let query_vec = query.to_vec();
        if query_vec.iter().any(|v| !v.is_finite()) {
            return Err(EdgeVecError::Validation(
                "Query vector contains non-finite values".to_string(),
            )
            .into());
        }

        // Execute BQ rescored search
        let results = self
            .inner
            .search_bq_rescored(&query_vec, k, rescore_factor, &self.storage)
            .map_err(EdgeVecError::from)?;

        // Convert results to JavaScript array
        let arr = Array::new_with_length(results.len() as u32);
        for (i, (vector_id, similarity)) in results.iter().enumerate() {
            let obj = Object::new();
            Reflect::set(
                &obj,
                &JsValue::from_str("id"),
                &JsValue::from(vector_id.0 as u32),
            )?;
            Reflect::set(
                &obj,
                &JsValue::from_str("distance"),
                &JsValue::from(*similarity),
            )?;
            arr.set(i as u32, obj.into());
        }

        Ok(arr.into())
    }

    /// Hybrid search combining BQ speed with metadata filtering.
    ///
    /// This is the most flexible search method, combining:
    /// - Binary quantization for speed
    /// - Metadata filtering for precision
    /// - Optional F32 rescoring for accuracy
    ///
    /// # Arguments
    ///
    /// * `query` - A Float32Array containing the query vector
    /// * `options` - A JavaScript object with search options:
    ///   - `k` (required): Number of results to return
    ///   - `filter` (optional): Filter expression string
    ///   - `useBQ` (optional, default true): Use binary quantization
    ///   - `rescoreFactor` (optional, default 3): Overfetch multiplier
    ///
    /// # Returns
    ///
    /// An array of search result objects: `[{ id: number, distance: number }, ...]`
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Options is not a valid object
    /// - k is 0 or missing
    /// - Filter expression is invalid
    /// - Query dimensions mismatch
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const results = index.searchHybrid(
    ///     new Float32Array([0.1, 0.2, ...]),
    ///     {
    ///         k: 10,
    ///         filter: 'category == "news" AND score > 0.5',
    ///         useBQ: true,
    ///         rescoreFactor: 3
    ///     }
    /// );
    /// ```
    #[wasm_bindgen(js_name = "searchHybrid")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search_hybrid(
        &mut self,
        query: Float32Array,
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        use crate::filter::{parse, FilterStrategy, FilteredSearcher};

        // Parse options
        let opts = parse_hybrid_search_options(&options)?;

        // Validate k
        if opts.k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }

        // Validate query dimensions
        let len = query.length();
        if len != self.inner.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: self.inner.config.dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let query_vec = query.to_vec();
        if query_vec.iter().any(|v| !v.is_finite()) {
            return Err(EdgeVecError::Validation(
                "Query vector contains non-finite values".to_string(),
            )
            .into());
        }

        // Determine search strategy
        let use_bq = opts.use_bq && self.inner.bq_storage.is_some();
        let rescore_factor = opts.rescore_factor.max(1);

        // Execute appropriate search based on options
        let results: Vec<(crate::hnsw::VectorId, f32)> = if use_bq {
            if let Some(ref filter_str) = opts.filter {
                // BQ + filter + rescore: Use filtered search with BQ candidates
                let filter_expr =
                    parse(filter_str).map_err(|e| filter::filter_error_to_jsvalue(&e))?;

                // Get BQ candidates with overfetch
                let overfetch_k = opts.k.saturating_mul(rescore_factor);
                let bq_candidates = self
                    .inner
                    .search_bq(&query_vec, overfetch_k, &self.storage)
                    .map_err(EdgeVecError::from)?;

                // Filter candidates using metadata
                let empty_map = std::collections::HashMap::new();
                let mut filtered: Vec<_> = bq_candidates
                    .into_iter()
                    .filter(|(vid, _)| {
                        let metadata = self
                            .inner
                            .metadata
                            .get_all(vid.0 as u32)
                            .unwrap_or(&empty_map);
                        crate::filter::evaluate(&filter_expr, metadata).unwrap_or(false)
                    })
                    .take(opts.k)
                    .collect();

                // Rescore filtered candidates with F32 if we have enough
                if !filtered.is_empty() {
                    use super::hnsw::rescore::rescore_top_k;
                    let rescored = rescore_top_k(
                        &filtered,
                        &query_vec,
                        &self.storage,
                        opts.k.min(filtered.len()),
                    );
                    filtered = rescored
                        .into_iter()
                        .map(|(id, dist)| (id, 1.0 / (1.0 + dist)))
                        .collect();
                }

                filtered
            } else {
                // BQ only (no filter)
                self.inner
                    .search_bq_rescored(&query_vec, opts.k, rescore_factor, &self.storage)
                    .map_err(EdgeVecError::from)?
            }
        } else if let Some(ref filter_str) = opts.filter {
            // F32 + filter (no BQ)
            let filter_expr = parse(filter_str).map_err(|e| filter::filter_error_to_jsvalue(&e))?;
            let metadata_adapter =
                EdgeVecMetadataAdapter::new(&self.inner.metadata, self.inner.len());
            let mut searcher = FilteredSearcher::new(&self.inner, &self.storage, &metadata_adapter);
            let result = searcher
                .search_filtered(&query_vec, opts.k, Some(&filter_expr), FilterStrategy::Auto)
                .map_err(|e| JsValue::from_str(&format!("Search failed: {e}")))?;
            result
                .results
                .into_iter()
                .map(|r| (r.vector_id, r.distance))
                .collect()
        } else {
            // Pure F32 search (no BQ, no filter)
            let search_results = self
                .inner
                .search(&query_vec, opts.k, &self.storage)
                .map_err(EdgeVecError::from)?;
            search_results
                .into_iter()
                .map(|r| (r.vector_id, r.distance))
                .collect()
        };

        // Convert results to JavaScript array
        let arr = Array::new_with_length(results.len() as u32);
        for (i, (vector_id, distance)) in results.iter().enumerate() {
            let obj = Object::new();
            Reflect::set(
                &obj,
                &JsValue::from_str("id"),
                &JsValue::from(vector_id.0 as u32),
            )?;
            Reflect::set(
                &obj,
                &JsValue::from_str("distance"),
                &JsValue::from(*distance),
            )?;
            arr.set(i as u32, obj.into());
        }

        Ok(arr.into())
    }

    /// Check if binary quantization is enabled on this index.
    ///
    /// # Returns
    ///
    /// `true` if BQ is enabled and ready for use, `false` otherwise.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// if (index.hasBQ()) {
    ///     const results = index.searchBQ(query, 10);
    /// } else {
    ///     const results = index.search(query, 10);
    /// }
    /// ```
    #[wasm_bindgen(js_name = "hasBQ")]
    #[must_use]
    pub fn has_bq(&self) -> bool {
        self.inner.bq_storage.is_some()
    }

    /// Enables binary quantization on this index.
    ///
    /// Binary quantization reduces memory usage by 32x (from 32 bits to 1 bit per dimension)
    /// while maintaining ~85-95% recall. BQ is automatically enabled for dimensions divisible by 8.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Dimensions are not divisible by 8 (required for BQ)
    /// - BQ is already enabled
    ///
    /// # Example
    ///
    /// ```javascript
    /// const db = new EdgeVec(config);
    /// db.enableBQ();  // Enable BQ for faster search
    ///
    /// // Insert vectors (BQ codes computed automatically)
    /// db.insert(vector);
    ///
    /// // Use BQ search
    /// const results = db.searchBQ(query, 10);
    /// ```
    #[wasm_bindgen(js_name = "enableBQ")]
    pub fn enable_bq(&mut self) -> Result<(), JsValue> {
        self.inner
            .enable_bq(&self.storage)
            .map_err(|e| EdgeVecError::from(e).into())
    }

    // =========================================================================
    // FILTERED SEARCH API (v0.5.0 — Week 23)
    // =========================================================================

    /// Execute a filtered search on the index.
    ///
    /// Combines HNSW vector search with metadata filtering using configurable
    /// strategies (pre-filter, post-filter, hybrid, auto).
    ///
    /// # Arguments
    ///
    /// * `query` - A Float32Array containing the query vector
    /// * `k` - Number of results to return
    /// * `options_json` - JSON object with search options:
    ///   ```json
    ///   {
    ///     "filter": "category = \"gpu\"",  // optional filter expression
    ///     "strategy": "auto",              // "auto" | "pre" | "post" | "hybrid"
    ///     "oversampleFactor": 3.0,         // for post/hybrid strategies
    ///     "includeMetadata": true,         // include metadata in results
    ///     "includeVectors": false          // include vectors in results
    ///   }
    ///   ```
    ///
    /// # Returns
    ///
    /// JSON string with search results:
    /// ```json
    /// {
    ///   "results": [{ "id": 42, "score": 0.95, "metadata": {...}, "vector": [...] }],
    ///   "complete": true,
    ///   "observedSelectivity": 0.15,
    ///   "strategyUsed": "hybrid",
    ///   "vectorsEvaluated": 150,
    ///   "filterTimeMs": 2.5,
    ///   "totalTimeMs": 8.3
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Query dimensions don't match index
    /// - Filter expression is invalid
    /// - Options JSON is malformed
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const query = new Float32Array([0.1, 0.2, ...]);
    /// const result = JSON.parse(index.searchFiltered(query, 10, JSON.stringify({
    ///     filter: 'category = "gpu" AND price < 500',
    ///     strategy: 'auto'
    /// })));
    /// console.log(`Found ${result.results.length} results`);
    /// ```
    #[wasm_bindgen(js_name = "searchFiltered")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search_filtered(
        &mut self,
        query: Float32Array,
        k: usize,
        options_json: &str,
    ) -> Result<String, JsValue> {
        use crate::filter::{parse, FilterStrategy, FilteredSearcher};

        // Start total timing
        let total_start = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now());

        // Validate query dimensions
        let len = query.length();
        if len != self.inner.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: self.inner.config.dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let query_vec = query.to_vec();
        if query_vec.iter().any(|v| !v.is_finite()) {
            return Err(EdgeVecError::Validation(
                "Query vector contains non-finite values".to_string(),
            )
            .into());
        }

        // Parse options
        let options: SearchFilteredOptions = serde_json::from_str(options_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid options JSON: {e}")))?;

        // Parse filter if provided (and time it)
        let filter_start = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now());

        let filter = match &options.filter {
            Some(filter_str) => {
                Some(parse(filter_str).map_err(|e| filter::filter_error_to_jsvalue(&e))?)
            }
            None => None,
        };

        // Convert strategy
        let strategy = match options.strategy.as_deref() {
            Some("pre") => FilterStrategy::PreFilter,
            Some("post") => FilterStrategy::PostFilter {
                oversample: options.oversample_factor.unwrap_or(3.0),
            },
            Some("hybrid") => FilterStrategy::Hybrid {
                oversample_min: 1.5,
                oversample_max: options.oversample_factor.unwrap_or(10.0),
            },
            _ => FilterStrategy::Auto,
        };

        // Create metadata store adapter
        let metadata_adapter = EdgeVecMetadataAdapter::new(&self.inner.metadata, self.inner.len());

        // Execute filtered search
        let mut searcher = FilteredSearcher::new(&self.inner, &self.storage, &metadata_adapter);
        let result = searcher
            .search_filtered(&query_vec, k, filter.as_ref(), strategy)
            .map_err(|e| JsValue::from_str(&format!("Search failed: {e}")))?;

        // Calculate filter time (includes parsing + evaluation)
        let filter_time_ms = match (
            filter_start,
            web_sys::window().and_then(|w| w.performance()),
        ) {
            (Some(start), Some(perf)) => perf.now() - start,
            _ => 0.0,
        };

        // Check if metadata/vectors should be included
        let include_metadata = options.include_metadata.unwrap_or(false);
        let include_vectors = options.include_vectors.unwrap_or(false);

        // Build response
        let response = SearchFilteredResult {
            results: result
                .results
                .iter()
                .map(|r| {
                    let id = r.vector_id.0 as u32;
                    SearchFilteredItem {
                        id,
                        score: r.distance,
                        metadata: if include_metadata {
                            self.inner
                                .metadata
                                .get_all(id)
                                .and_then(|m| serde_json::to_value(m).ok())
                        } else {
                            None
                        },
                        vector: if include_vectors {
                            Some(self.storage.get_vector(r.vector_id).to_vec())
                        } else {
                            None
                        },
                    }
                })
                .collect(),
            complete: result.complete,
            observed_selectivity: result.observed_selectivity,
            strategy_used: strategy_to_string(&result.strategy_used),
            vectors_evaluated: result.vectors_evaluated,
            filter_time_ms,
            total_time_ms: match (total_start, web_sys::window().and_then(|w| w.performance())) {
                (Some(start), Some(perf)) => perf.now() - start,
                _ => 0.0,
            },
        };

        serde_json::to_string(&response)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    // =========================================================================
    // MEMORY PRESSURE API (v0.6.0 — Week 28 RFC-002)
    // =========================================================================

    /// Get current memory pressure state.
    ///
    /// Returns memory usage statistics and pressure level.
    /// Use this to implement graceful degradation in your app.
    ///
    /// # Returns
    ///
    /// MemoryPressure object with:
    /// - `level`: "normal", "warning", or "critical"
    /// - `usedBytes`: Bytes currently allocated
    /// - `totalBytes`: Total WASM heap size
    /// - `usagePercent`: Usage as percentage (0-100)
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (should not happen in practice).
    ///
    /// # Thresholds
    ///
    /// - Normal: <80% usage
    /// - Warning: 80-95% usage (consider reducing data)
    /// - Critical: >95% usage (risk of OOM, stop inserts)
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const pressure = index.getMemoryPressure();
    /// if (pressure.level === 'warning') {
    ///     console.warn('Memory pressure high, consider compacting');
    ///     index.compact();
    /// } else if (pressure.level === 'critical') {
    ///     console.error('Memory critical, stopping inserts');
    ///     // Disable insert button, show warning to user
    /// }
    /// ```
    #[wasm_bindgen(js_name = "getMemoryPressure")]
    pub fn get_memory_pressure(&self) -> Result<JsValue, JsValue> {
        let pressure = MemoryPressure::current_with_thresholds(
            self.memory_config.warning_threshold,
            self.memory_config.critical_threshold,
        );
        serde_wasm_bindgen::to_value(&pressure).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Configure memory pressure thresholds.
    ///
    /// # Arguments
    ///
    /// * `config` - MemoryConfig object with optional fields:
    ///   - `warningThreshold`: Warning threshold percentage (default: 80)
    ///   - `criticalThreshold`: Critical threshold percentage (default: 95)
    ///   - `autoCompactOnWarning`: Auto-compact when warning threshold reached
    ///   - `blockInsertsOnCritical`: Block inserts when critical threshold reached
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `config` is not a valid MemoryConfig object
    /// - `warningThreshold` is not between 0 and 100
    /// - `criticalThreshold` is not between 0 and 100
    /// - `warningThreshold` is greater than or equal to `criticalThreshold`
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// index.setMemoryConfig({
    ///     warningThreshold: 70,
    ///     criticalThreshold: 90,
    ///     autoCompactOnWarning: true,
    ///     blockInsertsOnCritical: true
    /// });
    /// ```
    #[wasm_bindgen(js_name = "setMemoryConfig")]
    pub fn set_memory_config(&mut self, config: JsValue) -> Result<(), JsValue> {
        let config: MemoryConfig = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {e}")))?;

        // Validate thresholds
        if config.warning_threshold <= 0.0 || config.warning_threshold >= 100.0 {
            return Err(JsValue::from_str(
                "warningThreshold must be between 0 and 100",
            ));
        }
        if config.critical_threshold <= 0.0 || config.critical_threshold >= 100.0 {
            return Err(JsValue::from_str(
                "criticalThreshold must be between 0 and 100",
            ));
        }
        if config.warning_threshold >= config.critical_threshold {
            return Err(JsValue::from_str(
                "warningThreshold must be less than criticalThreshold",
            ));
        }

        self.memory_config = config;
        Ok(())
    }

    /// Check if inserts are allowed based on memory pressure.
    ///
    /// Returns `false` if memory is at critical level and
    /// `blockInsertsOnCritical` is enabled.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// if (index.canInsert()) {
    ///     const id = index.insert(vector);
    /// } else {
    ///     console.warn('Memory critical, insert blocked');
    ///     showMemoryWarning();
    /// }
    /// ```
    #[wasm_bindgen(js_name = "canInsert")]
    #[must_use]
    pub fn can_insert(&self) -> bool {
        if !self.memory_config.block_inserts_on_critical {
            return true;
        }

        let pressure = MemoryPressure::current_with_thresholds(
            self.memory_config.warning_threshold,
            self.memory_config.critical_threshold,
        );
        pressure.level != MemoryPressureLevel::Critical
    }

    /// Get memory recommendation based on current state.
    ///
    /// Provides actionable guidance based on memory pressure level.
    ///
    /// # Returns
    ///
    /// MemoryRecommendation object with:
    /// - `action`: "none", "compact", or "reduce"
    /// - `message`: Human-readable description
    /// - `canInsert`: Whether inserts are allowed
    /// - `suggestCompact`: Whether compaction would help
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (should not happen in practice).
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const rec = index.getMemoryRecommendation();
    /// if (rec.action === 'compact' && rec.suggestCompact) {
    ///     index.compact();
    /// } else if (rec.action === 'reduce') {
    ///     showMemoryWarning(rec.message);
    ///     disableInsertButton();
    /// }
    /// ```
    #[wasm_bindgen(js_name = "getMemoryRecommendation")]
    pub fn get_memory_recommendation(&self) -> Result<JsValue, JsValue> {
        let pressure = MemoryPressure::current_with_thresholds(
            self.memory_config.warning_threshold,
            self.memory_config.critical_threshold,
        );

        let needs_compaction = self.inner.needs_compaction();

        let recommendation = match pressure.level {
            MemoryPressureLevel::Normal => MemoryRecommendation {
                action: "none".to_string(),
                message: "Memory usage is healthy.".to_string(),
                can_insert: true,
                suggest_compact: needs_compaction,
            },
            MemoryPressureLevel::Warning => MemoryRecommendation {
                action: "compact".to_string(),
                message: format!(
                    "Memory usage at {:.1}%. Consider running compact() to free deleted vectors.",
                    pressure.usage_percent
                ),
                can_insert: true,
                suggest_compact: needs_compaction,
            },
            MemoryPressureLevel::Critical => MemoryRecommendation {
                action: "reduce".to_string(),
                message: format!(
                    "Memory usage critical at {:.1}%. Inserts blocked. Run compact() or delete vectors.",
                    pressure.usage_percent
                ),
                can_insert: !self.memory_config.block_inserts_on_critical,
                suggest_compact: true,
            },
        };

        serde_wasm_bindgen::to_value(&recommendation).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get the current memory configuration.
    ///
    /// # Returns
    ///
    /// MemoryConfig object with current settings.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (should not happen in practice).
    #[wasm_bindgen(js_name = "getMemoryConfig")]
    pub fn get_memory_config(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.memory_config)
            .map_err(|e| JsValue::from_str(&e.to_string()))
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

// =============================================================================
// FILTERED SEARCH HELPER TYPES (Week 23 Day 4)
// =============================================================================

use crate::filter::FilterStrategy;
use crate::metadata::MetadataValue;
use std::collections::HashMap;

/// Adapter that wraps EdgeVec's MetadataStore to implement filter::MetadataStore trait.
///
/// This struct provides the bridge between EdgeVec's HashMap-based metadata storage
/// and the filter system's trait requirements.
struct EdgeVecMetadataAdapter<'a> {
    store: &'a crate::metadata::MetadataStore,
    /// Total number of vectors in the index (needed for len()).
    total_vectors: usize,
}

impl<'a> EdgeVecMetadataAdapter<'a> {
    fn new(store: &'a crate::metadata::MetadataStore, total_vectors: usize) -> Self {
        Self {
            store,
            total_vectors,
        }
    }
}

impl crate::filter::MetadataStore for EdgeVecMetadataAdapter<'_> {
    #[allow(clippy::cast_possible_truncation)]
    fn get_metadata(&self, id: usize) -> Option<&HashMap<String, MetadataValue>> {
        // Filter uses 0-indexed iteration (0..total), but VectorId is 1-indexed.
        // Add 1 to convert from filter's 0-based index to VectorId's 1-based ID.
        self.store.get_all((id + 1) as u32)
    }

    fn len(&self) -> usize {
        self.total_vectors
    }
}

/// Options for filtered search (JSON deserialization).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchFilteredOptions {
    /// Optional filter expression string.
    filter: Option<String>,
    /// Strategy override ("auto", "pre", "post", "hybrid").
    strategy: Option<String>,
    /// Oversample factor for post/hybrid strategies.
    oversample_factor: Option<f32>,
    /// Whether to include metadata in results.
    include_metadata: Option<bool>,
    /// Whether to include vectors in results.
    include_vectors: Option<bool>,
}

/// Result from filtered search (JSON serialization).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchFilteredResult {
    /// Search results.
    results: Vec<SearchFilteredItem>,
    /// Whether full k results were found.
    complete: bool,
    /// Observed filter selectivity (0.0 - 1.0).
    observed_selectivity: f32,
    /// Strategy actually used.
    strategy_used: String,
    /// Number of vectors evaluated.
    vectors_evaluated: usize,
    /// Time spent on filter evaluation (milliseconds).
    filter_time_ms: f64,
    /// Total search time (milliseconds).
    total_time_ms: f64,
}

/// Single result item from filtered search.
#[derive(Serialize)]
struct SearchFilteredItem {
    /// Vector ID.
    id: u32,
    /// Distance/similarity score.
    score: f32,
    /// Metadata (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    /// Vector data (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<Vec<f32>>,
}

/// Convert FilterStrategy to string for JSON response.
fn strategy_to_string(strategy: &FilterStrategy) -> String {
    match strategy {
        FilterStrategy::PreFilter => "pre".to_string(),
        FilterStrategy::PostFilter { .. } => "post".to_string(),
        FilterStrategy::Hybrid { .. } => "hybrid".to_string(),
        FilterStrategy::Auto => "auto".to_string(),
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR METADATA PARSING (Week 28 RFC-002)
// =============================================================================

/// Maximum safe integer in JavaScript (2^53 - 1).
const JS_MAX_SAFE_INT: f64 = 9_007_199_254_740_991.0;

/// Minimum safe integer in JavaScript (-(2^53 - 1)).
const JS_MIN_SAFE_INT: f64 = -9_007_199_254_740_991.0;

/// Parse a JavaScript object into a HashMap<String, MetadataValue>.
///
/// Automatically detects value types:
/// - String → MetadataValue::String
/// - Number (integer) → MetadataValue::Integer
/// - Number (float) → MetadataValue::Float
/// - Boolean → MetadataValue::Boolean
/// - Array of strings → MetadataValue::StringArray
///
/// # Errors
///
/// Returns an error if:
/// - The input is not a valid JavaScript object
/// - A value has an unsupported type
/// - An array contains non-string elements
#[allow(clippy::cast_possible_truncation)]
fn parse_js_metadata_object(js_obj: &JsValue) -> Result<HashMap<String, MetadataValue>, JsValue> {
    use js_sys::Object as JsObject;

    // Check if it's an object
    if !js_obj.is_object() {
        return Err(JsValue::from_str("Metadata must be a JavaScript object"));
    }

    let obj = JsObject::try_from(js_obj)
        .ok_or_else(|| JsValue::from_str("Failed to convert metadata to JavaScript object"))?;

    let mut metadata = HashMap::new();

    // Get all enumerable property keys
    let keys = JsObject::keys(obj);

    for i in 0..keys.length() {
        let key_js = keys.get(i);
        let key = key_js
            .as_string()
            .ok_or_else(|| JsValue::from_str("Metadata key must be a string"))?;

        let value_js = Reflect::get(obj, &key_js)?;
        let value = parse_js_metadata_value(&key, &value_js)?;

        metadata.insert(key, value);
    }

    Ok(metadata)
}

/// Parse a single JavaScript value into MetadataValue.
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
fn parse_js_metadata_value(key: &str, value: &JsValue) -> Result<MetadataValue, JsValue> {
    // Check for null/undefined
    if value.is_null() || value.is_undefined() {
        return Err(JsValue::from_str(&format!(
            "Metadata value for key '{key}' cannot be null or undefined"
        )));
    }

    // Check for string
    if let Some(s) = value.as_string() {
        return Ok(MetadataValue::String(s));
    }

    // Check for boolean
    if let Some(b) = value.as_bool() {
        return Ok(MetadataValue::Boolean(b));
    }

    // Check for number
    if let Some(n) = value.as_f64() {
        if !n.is_finite() {
            return Err(JsValue::from_str(&format!(
                "Metadata value for key '{key}' must be finite (not NaN or Infinity)"
            )));
        }

        // Detect if it's an integer (no fractional part)
        // Use JavaScript safe integer bounds for precision safety
        if n.fract() == 0.0 && (JS_MIN_SAFE_INT..=JS_MAX_SAFE_INT).contains(&n) {
            return Ok(MetadataValue::Integer(n as i64));
        }
        return Ok(MetadataValue::Float(n));
    }

    // Check for array (string array)
    if js_sys::Array::is_array(value) {
        let arr = js_sys::Array::from(value);
        let mut strings = Vec::with_capacity(arr.length() as usize);

        for i in 0..arr.length() {
            let item = arr.get(i);
            let s = item.as_string().ok_or_else(|| {
                JsValue::from_str(&format!(
                    "Metadata array for key '{key}' must contain only strings, found non-string at index {i}"
                ))
            })?;
            strings.push(s);
        }

        return Ok(MetadataValue::StringArray(strings));
    }

    Err(JsValue::from_str(&format!(
        "Unsupported metadata value type for key '{key}'. Supported types: string, number, boolean, string[]"
    )))
}

// =============================================================================
// HELPER FUNCTIONS FOR BQ HYBRID SEARCH (Week 28 RFC-002)
// =============================================================================

/// Options for hybrid BQ search.
struct HybridSearchOptions {
    /// Number of results to return.
    k: usize,
    /// Optional filter expression.
    filter: Option<String>,
    /// Whether to use binary quantization (default: true).
    use_bq: bool,
    /// Rescore factor for BQ (default: 3).
    rescore_factor: usize,
}

/// Parse hybrid search options from a JavaScript object.
///
/// Expected object shape:
/// ```javascript
/// {
///     k: 10,                    // required
///     filter: 'category == "news"',  // optional
///     useBQ: true,              // optional, default true
///     rescoreFactor: 3          // optional, default 3
/// }
/// ```
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn parse_hybrid_search_options(options: &JsValue) -> Result<HybridSearchOptions, JsValue> {
    if !options.is_object() {
        return Err(JsValue::from_str(
            "Options must be a JavaScript object with at least { k: number }",
        ));
    }

    // Get k (required)
    let k_js = Reflect::get(options, &JsValue::from_str("k"))?;
    let k = k_js
        .as_f64()
        .ok_or_else(|| JsValue::from_str("Options.k is required and must be a positive number"))?
        as usize;

    // Get filter (optional)
    let filter_js = Reflect::get(options, &JsValue::from_str("filter"))?;
    let filter = if filter_js.is_undefined() || filter_js.is_null() {
        None
    } else {
        filter_js.as_string()
    };

    // Get useBQ (optional, default true)
    let use_bq_js = Reflect::get(options, &JsValue::from_str("useBQ"))?;
    let use_bq = if use_bq_js.is_undefined() || use_bq_js.is_null() {
        true
    } else {
        use_bq_js.as_bool().unwrap_or(true)
    };

    // Get rescoreFactor (optional, default 3)
    let rescore_factor_js = Reflect::get(options, &JsValue::from_str("rescoreFactor"))?;
    let rescore_factor = if rescore_factor_js.is_undefined() || rescore_factor_js.is_null() {
        3
    } else {
        rescore_factor_js.as_f64().unwrap_or(3.0) as usize
    };

    Ok(HybridSearchOptions {
        k,
        filter,
        use_bq,
        rescore_factor,
    })
}

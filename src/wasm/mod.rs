//! WASM Bindings for EdgeVec.

use crate::error::EdgeVecError;
use crate::flat::BinaryFlatIndex;
use crate::hnsw::{GraphError, HnswConfig, HnswIndex};
#[cfg(feature = "sparse")]
use crate::hybrid::{FusionMethod, HybridSearchConfig, HybridSearcher};
use crate::metadata::validation::{validate_key, validate_value, MAX_KEYS_PER_VECTOR};
use crate::metadata::MetadataStore;
use crate::persistence::{chunking::ChunkIter, ChunkedWriter, PersistenceError};
#[cfg(feature = "sparse")]
use crate::sparse::{SparseSearcher, SparseStorage, SparseVector};
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

// =============================================================================
// Serialization Constants
// =============================================================================

/// Estimated header size for HNSW index serialization.
///
/// This accounts for the snapshot format header, config, and metadata.
const HNSW_HEADER_SIZE: usize = 64;

/// Estimated per-node graph overhead for HNSW serialization.
///
/// Each node stores neighbor lists across all layers. Average overhead
/// is approximately 64 bytes per vector for typical M=12 configuration.
const HNSW_NODE_OVERHEAD: usize = 64;

/// Default chunk size for streaming persistence (10MB).
///
/// This is optimized for browser memory constraints while maintaining
/// good throughput for IndexedDB writes.
const DEFAULT_CHUNK_SIZE: usize = 10 * 1024 * 1024;

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

/// Batch benchmark: Compare SIMD implementations searching through N vectors.
///
/// This is a realistic benchmark that simulates searching through a dataset:
/// - Accepts vectors from JavaScript (same path as real insertions)
/// - For each iteration, computes hamming distance from query to ALL vectors
/// - Compares new WASM SIMD128 vs current scalar fallback
///
/// # Arguments
///
/// * `vectors_js` - Array of Uint8Array vectors (created in JavaScript)
/// * `query_js` - Query vector as Uint8Array
/// * `iterations` - Number of full scans to perform
///
/// Returns JSON with throughput metrics:
/// ```json
/// {
///   "num_vectors": 10000,
///   "bytes_per_vector": 128,
///   "iterations": 100,
///   "new_ms": 1.23,
///   "current_ms": 3.45,
///   "speedup": 2.8,
///   "new_throughput": "8.1M vec/s",
///   "current_throughput": "2.9M vec/s"
/// }
/// ```
#[wasm_bindgen(js_name = "benchmarkHammingBatch")]
#[allow(clippy::needless_pass_by_value)] // wasm-bindgen requires pass-by-value for Uint8Array
#[allow(clippy::cast_possible_truncation)] // Intentional truncation for pseudo-random test data
pub fn benchmark_hamming_batch(
    vectors_js: &js_sys::Array,
    query_js: Uint8Array,
    iterations: usize,
) -> String {
    use crate::simd::popcount::simd_popcount_xor;

    // Convert JS vectors to Rust - same path as real insertions
    let vectors: Vec<Vec<u8>> = vectors_js
        .iter()
        .map(|v| Uint8Array::from(v).to_vec())
        .collect();

    let num_vectors = vectors.len();
    let bytes_per_vector = if num_vectors > 0 { vectors[0].len() } else { 0 };

    // Convert query vector
    let query: Vec<u8> = query_js.to_vec();

    let perf = web_sys::window().and_then(|w| w.performance());

    // Warmup
    for v in vectors.iter().take(100.min(num_vectors)) {
        let _ = crate::metric::simd::hamming_distance(&query, v);
        let _ = simd_popcount_xor(&query, v);
    }

    // Benchmark NEW: metric::simd::hamming_distance (WASM SIMD128)
    let start_new = perf.as_ref().map_or(0.0, web_sys::Performance::now);
    let mut total_dist_new: u64 = 0;
    for _ in 0..iterations {
        for v in &vectors {
            total_dist_new += u64::from(crate::metric::simd::hamming_distance(&query, v));
        }
    }
    let end_new = perf.as_ref().map_or(0.0, web_sys::Performance::now);

    // Benchmark CURRENT: simd::popcount::simd_popcount_xor (scalar fallback in WASM)
    let start_current = perf.as_ref().map_or(0.0, web_sys::Performance::now);
    let mut total_dist_current: u64 = 0;
    for _ in 0..iterations {
        for v in &vectors {
            total_dist_current += u64::from(simd_popcount_xor(&query, v));
        }
    }
    let end_current = perf.as_ref().map_or(0.0, web_sys::Performance::now);

    // Prevent optimizer removal
    if total_dist_new == 0 || total_dist_current == 0 {
        web_sys::console::log_1(&format!("sums: {total_dist_new} {total_dist_current}").into());
    }

    let new_ms = end_new - start_new;
    let current_ms = end_current - start_current;
    let speedup = current_ms / new_ms;

    let total_comparisons = num_vectors * iterations;
    #[allow(clippy::cast_precision_loss)]
    let new_throughput = (total_comparisons as f64) / (new_ms / 1000.0);
    #[allow(clippy::cast_precision_loss)]
    let current_throughput = (total_comparisons as f64) / (current_ms / 1000.0);

    // Format throughput with appropriate units
    let format_throughput = |t: f64| -> String {
        if t >= 1_000_000.0 {
            format!("{:.1}M vec/s", t / 1_000_000.0)
        } else if t >= 1_000.0 {
            format!("{:.1}K vec/s", t / 1_000.0)
        } else {
            format!("{:.0} vec/s", t)
        }
    };

    format!(
        r#"{{"num_vectors": {num_vectors}, "bytes_per_vector": {bytes_per_vector}, "iterations": {iterations}, "new_ms": {new_ms:.2}, "current_ms": {current_ms:.2}, "speedup": {speedup:.2}, "new_throughput": "{}", "current_throughput": "{}"}}"#,
        format_throughput(new_throughput),
        format_throughput(current_throughput)
    )
}

/// Vector storage type for EdgeVec.
///
/// Determines how vectors are stored and processed.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorType {
    /// Standard 32-bit floating point vectors.
    Float32 = 0,
    /// Binary vectors (1-bit per dimension, packed into bytes).
    /// Use with `metric = "hamming"`.
    Binary = 1,
}

/// Distance metric type for EdgeVec.
///
/// Determines how vector similarity is calculated.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricType {
    /// L2 Squared (Euclidean) distance.
    L2 = 0,
    /// Cosine similarity (converted to distance).
    Cosine = 1,
    /// Dot product (converted to distance).
    Dot = 2,
    /// Hamming distance (for binary vectors).
    Hamming = 3,
}

/// Index type for EdgeVec.
///
/// Determines the search algorithm and performance characteristics.
///
/// ## Performance Comparison
///
/// | Index Type | Insert | Search (1M) | Recall | Best For |
/// |------------|--------|-------------|--------|----------|
/// | Flat       | O(1) ~1μs | O(n) ~5-10ms | 100% (exact) | Real-time apps, <1M vectors |
/// | HNSW       | O(log n) ~2ms | O(log n) ~2ms | 90-95% | Large datasets, batch insert |
///
/// ## Example (JavaScript)
///
/// ```javascript
/// import { EdgeVecConfig, IndexType } from 'edgevec';
///
/// // Create a flat index for insert-heavy workloads
/// const config = new EdgeVecConfig(1024);
/// config.indexType = IndexType.Flat;
///
/// // Create an HNSW index for large-scale search (default)
/// const hnswConfig = new EdgeVecConfig(1024);
/// hnswConfig.indexType = IndexType.Hnsw; // This is the default
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JsIndexType {
    /// Brute force search (O(1) insert, O(n) search).
    ///
    /// Best for:
    /// - Insert-heavy workloads (semantic caching)
    /// - Datasets < 1M vectors
    /// - When 100% recall (exact search) is required
    Flat = 0,

    /// HNSW graph index (O(log n) insert, O(log n) search).
    ///
    /// Best for:
    /// - Large datasets (>1M vectors)
    /// - Read-heavy workloads
    /// - When approximate nearest neighbors is acceptable
    #[default]
    Hnsw = 1,
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
    vector_type: Option<VectorType>,
    index_type: Option<JsIndexType>,
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
            vector_type: None,
            index_type: None, // Defaults to HNSW
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

    /// Set distance metric ("l2", "cosine", "dot", "hamming").
    #[wasm_bindgen(setter)]
    pub fn set_metric(&mut self, metric: String) {
        self.metric = Some(metric);
    }

    /// Set distance metric using typed enum.
    ///
    /// # Example
    ///
    /// ```javascript
    /// const config = new EdgeVecConfig(768);
    /// config.setMetricType(MetricType.Cosine);
    /// ```
    #[wasm_bindgen(js_name = "setMetricType")]
    pub fn set_metric_type(&mut self, metric_type: MetricType) {
        let metric_str = match metric_type {
            MetricType::L2 => "l2",
            MetricType::Cosine => "cosine",
            MetricType::Dot => "dot",
            MetricType::Hamming => "hamming",
        };
        self.metric = Some(metric_str.to_string());
    }

    /// Set vector storage type.
    ///
    /// Use `VectorType.Binary` with `MetricType.Hamming` for binary vectors.
    ///
    /// # Example
    ///
    /// ```javascript
    /// const config = new EdgeVecConfig(1024);
    /// config.setVectorType(VectorType.Binary);
    /// config.setMetricType(MetricType.Hamming);
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_vector_type(&mut self, vt: VectorType) {
        self.vector_type = Some(vt);
        // Auto-set hamming metric for binary vectors if not already set
        if vt == VectorType::Binary && self.metric.is_none() {
            self.metric = Some("hamming".to_string());
        }
    }

    /// Get the configured vector type.
    #[wasm_bindgen(getter)]
    pub fn vector_type(&self) -> Option<VectorType> {
        self.vector_type
    }

    /// Set the index type (Flat or HNSW).
    ///
    /// - `Flat`: Brute force search (O(1) insert, O(n) search). Best for insert-heavy
    ///   workloads, datasets < 1M vectors, or when 100% recall is required.
    /// - `HNSW`: Graph-based search (O(log n) insert, O(log n) search). Best for
    ///   large datasets and read-heavy workloads.
    ///
    /// # Example
    ///
    /// ```javascript
    /// // For insert-heavy workloads (semantic caching)
    /// const config = new EdgeVecConfig(1024);
    /// config.indexType = IndexType.Flat;
    ///
    /// // For large-scale search (default)
    /// const config2 = new EdgeVecConfig(1024);
    /// config2.indexType = IndexType.Hnsw;
    /// ```
    #[wasm_bindgen(setter, js_name = "indexType")]
    pub fn set_index_type(&mut self, index_type: JsIndexType) {
        self.index_type = Some(index_type);
    }

    /// Get the configured index type.
    #[wasm_bindgen(getter, js_name = "indexType")]
    pub fn index_type(&self) -> JsIndexType {
        self.index_type.unwrap_or_default()
    }

    /// Check if this configuration uses a Flat index.
    #[wasm_bindgen(js_name = "isFlat")]
    pub fn is_flat(&self) -> bool {
        matches!(self.index_type, Some(JsIndexType::Flat))
    }

    /// Check if this configuration uses an HNSW index (default).
    #[wasm_bindgen(js_name = "isHnsw")]
    pub fn is_hnsw(&self) -> bool {
        !self.is_flat()
    }
}

/// Internal index variant for unified Flat/HNSW support.
///
/// This enum allows EdgeVec to wrap either a BinaryFlatIndex or an HnswIndex,
/// enabling a single API for both index types.
#[derive(Serialize, Deserialize)]
#[serde(tag = "variant_type")]
pub(crate) enum IndexVariant {
    /// HNSW graph index with separate storage.
    /// Box is used to reduce enum size (HnswIndex is 736 bytes vs BinaryFlatIndex 48 bytes).
    #[serde(rename = "hnsw")]
    Hnsw {
        index: Box<HnswIndex>,
        storage: VectorStorage,
    },
    /// Flat (brute-force) index with internal storage.
    #[serde(rename = "flat")]
    Flat { index: BinaryFlatIndex },
}

#[allow(dead_code)]
impl IndexVariant {
    /// Get dimensions (bits for binary, floats for HNSW).
    #[inline]
    pub fn dimensions(&self) -> u32 {
        match self {
            IndexVariant::Hnsw { index, .. } => index.config.dimensions,
            IndexVariant::Flat { index } => index.dimensions() as u32,
        }
    }

    /// Get number of vectors in the index.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            IndexVariant::Hnsw { index, .. } => index.len(),
            IndexVariant::Flat { index } => index.len(),
        }
    }

    /// Check if index is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get memory usage in bytes.
    ///
    /// Includes all vector storage buffers (f32, quantized, binary) plus index overhead.
    #[inline]
    pub fn memory_usage(&self) -> usize {
        match self {
            IndexVariant::Hnsw { index, storage } => {
                // Count all vector data buffers
                let vector_data = storage.data_f32.capacity() * std::mem::size_of::<f32>()
                    + storage.quantized_data.capacity()
                    + storage.binary_data.capacity();
                // Deleted flags (1 bit per vector, capacity in bits / 8)
                let deleted_bits = storage.deleted.capacity() / 8;
                vector_data + deleted_bits + index.memory_usage()
            }
            IndexVariant::Flat { index } => index.memory_usage(),
        }
    }

    /// Get estimated serialized size in bytes.
    #[inline]
    pub fn serialized_size(&self) -> usize {
        match self {
            IndexVariant::Hnsw { index, storage } => {
                // Header + vector data (f32, quantized, or binary) + graph overhead estimate
                let vector_data = storage.data_f32.len() * std::mem::size_of::<f32>()
                    + storage.quantized_data.len()
                    + storage.binary_data.len();
                HNSW_HEADER_SIZE + vector_data + index.len() * HNSW_NODE_OVERHEAD
            }
            IndexVariant::Flat { index } => index.serialized_size(),
        }
    }

    /// Check if this is an HNSW index.
    #[inline]
    pub fn is_hnsw(&self) -> bool {
        matches!(self, IndexVariant::Hnsw { .. })
    }

    /// Check if this is a Flat index.
    #[inline]
    pub fn is_flat(&self) -> bool {
        matches!(self, IndexVariant::Flat { .. })
    }

    /// Get HNSW config (only valid for HNSW variant).
    /// Returns None for Flat index.
    #[inline]
    pub fn hnsw_config(&self) -> Option<&HnswConfig> {
        match self {
            IndexVariant::Hnsw { index, .. } => Some(&index.config),
            IndexVariant::Flat { .. } => None,
        }
    }

    /// Try to get mutable HNSW index and storage.
    /// Returns error if this is a Flat index.
    #[inline]
    pub fn as_hnsw_mut(&mut self) -> Result<(&mut HnswIndex, &mut VectorStorage), EdgeVecError> {
        match self {
            IndexVariant::Hnsw { index, storage } => Ok((index, storage)),
            IndexVariant::Flat { .. } => Err(EdgeVecError::Validation(
                "This operation is only supported for HNSW index. Use IndexType.Hnsw in config."
                    .to_string(),
            )),
        }
    }

    /// Try to get immutable HNSW index and storage.
    /// Returns error if this is a Flat index.
    #[inline]
    pub fn as_hnsw(&self) -> Result<(&HnswIndex, &VectorStorage), EdgeVecError> {
        match self {
            IndexVariant::Hnsw { index, storage } => Ok((index, storage)),
            IndexVariant::Flat { .. } => Err(EdgeVecError::Validation(
                "This operation is only supported for HNSW index. Use IndexType.Hnsw in config."
                    .to_string(),
            )),
        }
    }

    /// Try to get mutable Flat index.
    /// Returns error if this is an HNSW index.
    #[inline]
    pub fn as_flat_mut(&mut self) -> Result<&mut BinaryFlatIndex, EdgeVecError> {
        match self {
            IndexVariant::Flat { index } => Ok(index),
            IndexVariant::Hnsw { .. } => Err(EdgeVecError::Validation(
                "This operation is only supported for Flat index. Use IndexType.Flat in config."
                    .to_string(),
            )),
        }
    }

    /// Try to get immutable Flat index.
    /// Returns error if this is an HNSW index.
    #[inline]
    pub fn as_flat(&self) -> Result<&BinaryFlatIndex, EdgeVecError> {
        match self {
            IndexVariant::Flat { index } => Ok(index),
            IndexVariant::Hnsw { .. } => Err(EdgeVecError::Validation(
                "This operation is only supported for Flat index. Use IndexType.Flat in config."
                    .to_string(),
            )),
        }
    }
}

/// The main EdgeVec database handle.
///
/// This struct is serializable for persistence via `postcard`.
/// The `liveness` field is skipped as it is runtime state.
///
/// Supports both Flat and HNSW index types via the `indexType` configuration.
/// Use `IndexType.Flat` for insert-heavy workloads with exact search,
/// or `IndexType.Hnsw` (default) for large-scale approximate search.
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
    /// The underlying index (either Flat or HNSW).
    inner: IndexVariant,
    /// Metadata store for attaching key-value pairs to vectors.
    #[serde(default)]
    metadata: MetadataStore,
    /// Memory pressure configuration (skipped during serialization).
    #[serde(skip, default)]
    memory_config: MemoryConfig,
    /// Sparse vector storage for hybrid search (Week 39).
    ///
    /// Initialized lazily via `init_sparse_storage()`.
    ///
    /// # Persistence Decision (v0.9.0)
    ///
    /// **Sparse storage is intentionally NOT persisted in v0.9.0 snapshots.**
    ///
    /// Rationale:
    /// - Sparse vectors are typically derived from BM25/TF-IDF tokenizers
    /// - Regenerating from raw text is preferred to maintaining two copies
    /// - Snapshot format stability: adding sparse would require major version bump
    /// - Users can serialize `SparseStorage` separately if needed
    ///
    /// Future versions may add optional sparse persistence via separate file.
    #[cfg(feature = "sparse")]
    #[serde(skip, default)]
    sparse_storage: Option<SparseStorage>,
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

        // Dispatch based on index type
        let inner = match config.index_type() {
            JsIndexType::Flat => {
                // Flat index: create BinaryFlatIndex directly
                let index =
                    BinaryFlatIndex::new(config.dimensions as usize).map_err(EdgeVecError::from)?;
                IndexVariant::Flat { index }
            }
            JsIndexType::Hnsw => {
                // HNSW index: create with full configuration
                let metric_code = match config.metric.as_deref() {
                    Some("cosine") => HnswConfig::METRIC_COSINE,
                    Some("dot") => HnswConfig::METRIC_DOT_PRODUCT,
                    Some("l2") | None => HnswConfig::METRIC_L2_SQUARED,
                    Some("hamming") => HnswConfig::METRIC_HAMMING,
                    Some(other) => {
                        return Err(
                            EdgeVecError::Validation(format!("Unknown metric: {other}")).into()
                        )
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

                // Validate Binary + non-Hamming incompatibility
                if config.vector_type == Some(VectorType::Binary)
                    && metric_code != HnswConfig::METRIC_HAMMING
                {
                    return Err(EdgeVecError::Validation(format!(
                        "VectorType::Binary requires metric='hamming'. Current metric is '{}'",
                        match metric_code {
                            HnswConfig::METRIC_L2_SQUARED => "l2",
                            HnswConfig::METRIC_COSINE => "cosine",
                            HnswConfig::METRIC_DOT_PRODUCT => "dot",
                            _ => "unknown",
                        }
                    ))
                    .into());
                }

                // Initialize storage (in-memory for now)
                let mut storage = VectorStorage::new(&hnsw_config, None);

                // Set up binary storage if:
                // 1. Explicit VectorType::Binary is specified, OR
                // 2. Metric is Hamming (implies binary vectors)
                if config.vector_type == Some(VectorType::Binary)
                    || metric_code == HnswConfig::METRIC_HAMMING
                {
                    storage
                        .set_storage_type(crate::storage::StorageType::Binary(config.dimensions));
                }

                let index = HnswIndex::new(hnsw_config, &storage).map_err(EdgeVecError::from)?;
                IndexVariant::Hnsw {
                    index: Box::new(index),
                    storage,
                }
            }
        };

        Ok(EdgeVec {
            inner,
            metadata: MetadataStore::new(),
            memory_config: MemoryConfig::default(),
            #[cfg(feature = "sparse")]
            sparse_storage: None,
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
        match &mut self.inner {
            IndexVariant::Hnsw { index, storage } => {
                let len = vector.length();
                let dimensions = index.config.dimensions;
                if len != dimensions {
                    return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                        expected: dimensions as usize,
                        actual: len as usize,
                    })
                    .into());
                }

                let vec = vector.to_vec();

                // C-SRC-3: NaN/Infinity check runs in ALL builds (not just debug).
                // Without this, non-finite values silently corrupt the index in production WASM.
                if vec.iter().any(|v| !v.is_finite()) {
                    return Err(EdgeVecError::Validation(
                        "Vector contains non-finite values".to_string(),
                    )
                    .into());
                }

                let id = index.insert(&vec, storage).map_err(EdgeVecError::from)?;

                track_vector_insert(dimensions);

                if id.0 > u64::from(u32::MAX) {
                    return Err(
                        EdgeVecError::Validation("Vector ID overflowed u32".to_string()).into(),
                    );
                }
                Ok(id.0 as u32)
            }
            IndexVariant::Flat { .. } => Err(EdgeVecError::Validation(
                "insert() not supported for Flat index. Use insertBinary() instead.".to_string(),
            )
            .into()),
        }
    }

    // =========================================================================
    // BINARY VECTOR API (v0.6.0)
    // =========================================================================

    /// Inserts a pre-packed binary vector into the index.
    ///
    /// This method is for binary vectors (1-bit quantized) using Hamming distance.
    /// Use this when you have pre-quantized data (e.g., from Turso's `f1bit_blob`).
    ///
    /// # Arguments
    ///
    /// * `vector` - A Uint8Array containing packed binary data. Length must equal
    ///   `ceil(dimensions / 8)` bytes.
    ///
    /// # Returns
    ///
    /// The assigned Vector ID (u32).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Storage is not in Binary mode (metric != "hamming")
    /// - Byte length doesn't match expected bytes for dimensions
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const config = new EdgeVecConfig(1024); // 1024 bits = 128 bytes
    /// config.metric = 'hamming';
    /// const db = new EdgeVec(config);
    ///
    /// // Direct binary insertion (e.g., from Turso f1bit_blob)
    /// const binaryVector = new Uint8Array(128); // 1024 bits packed
    /// const id = db.insertBinary(binaryVector);
    /// ```
    #[wasm_bindgen(js_name = "insertBinary")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn insert_binary(&mut self, vector: Uint8Array) -> Result<u32, JsValue> {
        match &mut self.inner {
            IndexVariant::Hnsw { index, storage } => {
                // Validate metric is Hamming for HNSW
                if index.config.metric != HnswConfig::METRIC_HAMMING {
                    return Err(EdgeVecError::Validation(
                        "insertBinary requires metric='hamming'. Current metric is not Hamming."
                            .to_string(),
                    )
                    .into());
                }

                let expected_bytes = ((index.config.dimensions + 7) / 8) as usize;
                let len = vector.length() as usize;

                if len != expected_bytes {
                    return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                        expected: expected_bytes,
                        actual: len,
                    })
                    .into());
                }

                let vec = vector.to_vec();
                let id = index
                    .insert_binary(&vec, storage)
                    .map_err(EdgeVecError::from)?;

                if id.0 > u64::from(u32::MAX) {
                    return Err(
                        EdgeVecError::Validation("Vector ID overflowed u32".to_string()).into(),
                    );
                }
                Ok(id.0 as u32)
            }
            IndexVariant::Flat { index } => {
                // Validate dimensions before insert to return proper error instead of panic
                let expected_bytes = index.bytes_per_vector();
                let len = vector.length() as usize;

                if len != expected_bytes {
                    return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                        expected: expected_bytes,
                        actual: len,
                    })
                    .into());
                }

                let vec = vector.to_vec();
                let id = index.insert(&vec).map_err(EdgeVecError::from)?;

                // Check for u32 overflow (consistent with HNSW path)
                if id.0 > u64::from(u32::MAX) {
                    return Err(
                        EdgeVecError::Validation("Vector ID overflowed u32".to_string()).into(),
                    );
                }
                Ok(id.0 as u32)
            }
        }
    }

    /// Inserts an f32 vector with automatic binary quantization.
    ///
    /// The vector is converted to binary (1 bit per dimension) using sign quantization:
    /// - Positive values → 1
    /// - Non-positive values → 0
    ///
    /// # Arguments
    ///
    /// * `vector` - A Float32Array containing the vector data (must match dimensions).
    ///
    /// # Returns
    ///
    /// The assigned Vector ID (u32).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Storage is not in Binary mode (metric != "hamming")
    /// - Dimensions don't match
    /// - Vector contains NaNs
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const config = new EdgeVecConfig(1024);
    /// config.metric = 'hamming';
    /// const db = new EdgeVec(config);
    ///
    /// // Insert f32 vector with automatic binary quantization
    /// const f32Vector = new Float32Array(1024).fill(0.5); // Gets quantized to all 1s
    /// const id = db.insertWithBq(f32Vector);
    /// ```
    #[wasm_bindgen(js_name = "insertWithBq")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn insert_with_bq(&mut self, vector: Float32Array) -> Result<u32, JsValue> {
        match &mut self.inner {
            IndexVariant::Hnsw { index, storage } => {
                // Validate metric is Hamming
                if index.config.metric != HnswConfig::METRIC_HAMMING {
                    return Err(EdgeVecError::Validation(
                        "insertWithBq requires metric='hamming'. Current metric is not Hamming."
                            .to_string(),
                    )
                    .into());
                }

                let len = vector.length();
                if len != index.config.dimensions {
                    return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                        expected: index.config.dimensions as usize,
                        actual: len as usize,
                    })
                    .into());
                }

                let vec = vector.to_vec();

                // C-SRC-3: NaN/Infinity check runs in ALL builds (not just debug).
                if vec.iter().any(|v| !v.is_finite()) {
                    return Err(EdgeVecError::Validation(
                        "Vector contains non-finite values".to_string(),
                    )
                    .into());
                }

                let id = index
                    .insert_with_bq(&vec, storage)
                    .map_err(EdgeVecError::from)?;

                if id.0 > u64::from(u32::MAX) {
                    return Err(
                        EdgeVecError::Validation("Vector ID overflowed u32".to_string()).into(),
                    );
                }
                Ok(id.0 as u32)
            }
            IndexVariant::Flat { .. } => Err(EdgeVecError::Validation(
                "insertWithBq() not supported for Flat index. Use insertBinary() instead."
                    .to_string(),
            )
            .into()),
        }
    }

    /// Searches for nearest neighbors using a binary query vector.
    ///
    /// Uses Hamming distance to find the K most similar binary vectors.
    ///
    /// # Arguments
    ///
    /// * `query` - A Uint8Array containing the binary query vector.
    /// * `k` - The number of neighbors to return.
    ///
    /// # Returns
    ///
    /// An array of objects: `[{ id: u32, score: f32 }, ...]` where `score` is
    /// the Hamming distance (number of differing bits).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Storage is not in Binary mode (metric != "hamming")
    /// - Query byte length doesn't match expected
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const config = new EdgeVecConfig(1024);
    /// config.metric = 'hamming';
    /// const db = new EdgeVec(config);
    ///
    /// // ... insert binary vectors ...
    ///
    /// const queryBinary = new Uint8Array(128);
    /// const results = db.searchBinary(queryBinary, 10);
    /// results.forEach(r => console.log(`ID: ${r.id}, Hamming Distance: ${r.score}`));
    /// ```
    #[wasm_bindgen(js_name = "searchBinary")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search_binary(&self, query: Uint8Array, k: usize) -> Result<JsValue, JsValue> {
        match &self.inner {
            IndexVariant::Hnsw { index, storage } => {
                // Validate metric is Hamming for HNSW
                if index.config.metric != HnswConfig::METRIC_HAMMING {
                    return Err(EdgeVecError::Validation(
                        "searchBinary requires metric='hamming'. Current metric is not Hamming."
                            .to_string(),
                    )
                    .into());
                }

                let expected_bytes = ((index.config.dimensions + 7) / 8) as usize;
                let len = query.length() as usize;

                if len != expected_bytes {
                    return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                        expected: expected_bytes,
                        actual: len,
                    })
                    .into());
                }

                let vec = query.to_vec();
                let results = index
                    .search_binary(&vec, k, storage)
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
            IndexVariant::Flat { index } => {
                // Validate dimensions before search to return proper error instead of panic
                let expected_bytes = index.bytes_per_vector();
                let len = query.length() as usize;

                if len != expected_bytes {
                    return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                        expected: expected_bytes,
                        actual: len,
                    })
                    .into());
                }

                let vec = query.to_vec();
                let results = index.search(&vec, k).map_err(EdgeVecError::from)?;

                let arr = Array::new_with_length(results.len() as u32);
                for (i, result) in results.iter().enumerate() {
                    let obj = Object::new();
                    Reflect::set(
                        &obj,
                        &JsValue::from_str("id"),
                        &JsValue::from(result.id.0 as u32),
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
        }
    }

    /// Searches binary vectors with a custom ef_search parameter.
    ///
    /// This allows tuning the recall/speed tradeoff per-query:
    /// - Lower ef_search = faster, lower recall
    /// - Higher ef_search = slower, higher recall
    ///
    /// # Arguments
    ///
    /// * `query` - A Uint8Array containing the binary query vector.
    /// * `k` - The number of neighbors to return.
    /// * `ef_search` - Size of dynamic candidate list (must be >= k).
    ///
    /// # Returns
    ///
    /// An array of objects: `[{ id: u32, score: f32 }, ...]`
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// // Low ef_search = fast, ~90% recall
    /// const fastResults = db.searchBinaryWithEf(query, 10, 20);
    ///
    /// // High ef_search = slower, ~99% recall
    /// const accurateResults = db.searchBinaryWithEf(query, 10, 200);
    /// ```
    #[wasm_bindgen(js_name = "searchBinaryWithEf")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search_binary_with_ef(
        &self,
        query: Uint8Array,
        k: usize,
        ef_search: usize,
    ) -> Result<JsValue, JsValue> {
        // HNSW-only: ef_search parameter only applies to HNSW
        let (index, storage) = self.inner.as_hnsw()?;

        // Validate metric is Hamming
        if index.config.metric != HnswConfig::METRIC_HAMMING {
            return Err(EdgeVecError::Validation(
                "searchBinaryWithEf requires metric='hamming'. Current metric is not Hamming."
                    .to_string(),
            )
            .into());
        }

        let expected_bytes = ((index.config.dimensions + 7) / 8) as usize;
        let len = query.length() as usize;

        if len != expected_bytes {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: expected_bytes,
                actual: len,
            })
            .into());
        }

        let vec = query.to_vec();

        let results = index
            .search_binary_with_ef(&vec, k, ef_search, storage)
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

    /// Searches binary vectors with optional metadata filtering.
    ///
    /// Combines binary vector search (Hamming distance) with metadata filtering.
    ///
    /// # Arguments
    ///
    /// * `query` - Binary query vector as Uint8Array (packed bits, ceil(dimensions/8) bytes)
    /// * `k` - Maximum number of results to return
    /// * `options_json` - JSON string with search options:
    ///   - `filter`: Optional SQL-like filter expression (e.g., `"category = \"gpu\""`)
    ///   - `strategy`: Filter strategy - `"auto"`, `"pre"`, `"post"`, or `"hybrid"`
    ///   - `oversample_factor`: Oversample factor for post/hybrid strategies (default: 3.0)
    ///   - `include_metadata`: Whether to include metadata in results (default: false)
    ///
    /// # Returns
    ///
    /// JSON string containing search results with Hamming distances.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Storage is not in Binary mode (metric != "hamming")
    /// - Query byte length doesn't match expected dimensions
    /// - Options JSON is invalid
    /// - Filter expression parsing fails
    /// - Search fails
    ///
    /// # Example
    ///
    /// ```javascript
    /// const queryBinary = new Uint8Array(128); // 1024 bits
    /// const result = JSON.parse(db.searchBinaryFiltered(queryBinary, 10, JSON.stringify({
    ///   filter: 'category = "gpu"',
    ///   strategy: 'auto',
    ///   include_metadata: true
    /// })));
    /// console.log(`Found ${result.results.length} binary matches with filter`);
    /// ```
    #[wasm_bindgen(js_name = "searchBinaryFiltered")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn search_binary_filtered(
        &mut self,
        query: Uint8Array,
        k: usize,
        options_json: &str,
    ) -> Result<String, JsValue> {
        use crate::filter::{parse, FilterStrategy, FilteredSearcher};

        // HNSW-only: Filtered search requires HNSW index
        let (index, storage) = self.inner.as_hnsw()?;

        // Start total timing
        let total_start = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now());

        // Validate metric is Hamming
        if index.config.metric != HnswConfig::METRIC_HAMMING {
            return Err(EdgeVecError::Validation(
                "searchBinaryFiltered requires metric='hamming'. Current metric is not Hamming."
                    .to_string(),
            )
            .into());
        }

        // Validate query dimensions
        let expected_bytes = ((index.config.dimensions + 7) / 8) as usize;
        let len = query.length() as usize;

        if len != expected_bytes {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: expected_bytes,
                actual: len,
            })
            .into());
        }

        let query_vec = query.to_vec();

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
        // NOTE: For binary search, the strategy is always forced to PreFilter internally.
        // PostFilter and Hybrid could miss top-K results because binary search returns
        // a fixed candidate set based on Hamming distance. PreFilter ensures all matching
        // vectors are considered before ranking by Hamming distance.
        // The strategy parameter is accepted for API compatibility but ignored.
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
        let metadata_adapter = EdgeVecMetadataAdapter::new(&self.metadata, index.len());

        // Execute filtered binary search (always uses PreFilter internally)
        let mut searcher = FilteredSearcher::new(index, storage, &metadata_adapter);
        let result = searcher
            .search_binary_filtered(&query_vec, k, filter.as_ref(), strategy)
            .map_err(|e| JsValue::from_str(&format!("Binary filtered search failed: {e}")))?;

        // Calculate filter time (includes parsing + evaluation)
        let filter_time_ms = match (
            filter_start,
            web_sys::window().and_then(|w| w.performance()),
        ) {
            (Some(start), Some(perf)) => perf.now() - start,
            _ => 0.0,
        };

        // Check if metadata should be included
        let include_metadata = options.include_metadata.unwrap_or(false);

        // Build response (similar structure to searchFiltered, but no vector field since binary)
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
                            self.metadata
                                .get_all(id)
                                .and_then(|m| serde_json::to_value(m).ok())
                        } else {
                            None
                        },
                        vector: None, // Binary vectors not returned (different format)
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
        // HNSW-only: F32 batch insert requires HNSW index
        let (index, storage) = self.inner.as_hnsw_mut()?;

        let dim = index.config.dimensions as usize;
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

        // C-SRC-3: NaN/Infinity check runs in ALL builds (not just debug).
        // The is_finite() check is cheap relative to batch insert latency.
        if vec_data.iter().any(|v| !v.is_finite()) {
            return Err(
                EdgeVecError::Validation("Vectors contain non-finite values".to_string()).into(),
            );
        }

        let mut ids = Vec::with_capacity(count);

        for i in 0..count {
            let start = i * dim;
            let end = start + dim;
            let vector_slice = &vec_data[start..end];

            let id = index
                .insert(vector_slice, storage)
                .map_err(EdgeVecError::from)?;

            if id.0 > u64::from(u32::MAX) {
                return Err(
                    EdgeVecError::Validation("Vector ID overflowed u32".to_string()).into(),
                );
            }
            ids.push(id.0 as u32);
        }

        // Track memory allocation for the batch
        track_batch_insert(count, index.config.dimensions);

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
        // HNSW-only: F32 search is only for HNSW index
        let (index, storage) = self.inner.as_hnsw()?;

        let len = query.length();
        if len != index.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: index.config.dimensions as usize,
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

        let results = index.search(&vec, k, storage).map_err(EdgeVecError::from)?;

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
    pub fn save_stream(&self, chunk_size: Option<usize>) -> Result<PersistenceIterator, JsValue> {
        // HNSW-only: Chunked streaming is only implemented for HNSW
        let (index, storage) = self.inner.as_hnsw()?;
        let size = chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE);
        let writer = (storage, index);
        let iter = writer.export_chunked(size);

        // SAFETY: We transmute the lifetime from `'self` to `'static` because
        // wasm-bindgen cannot express borrowed return types to JavaScript.
        //
        // (a) Liveness guard: The returned `PersistenceIterator` holds a clone of
        //     `self.liveness` (an `Arc<AtomicBool>`). When `EdgeVec` is dropped, it
        //     sets `liveness` to `false`, allowing consumers to detect invalidation.
        //     This does NOT prevent use-after-free at the Rust level, but provides a
        //     JS-visible signal.
        //
        // (b) TOCTOU gap: There is a theoretical time-of-check-to-time-of-use gap
        //     between checking `liveness` and accessing the iterator data. A consumer
        //     could check `liveness == true`, then `EdgeVec` could be dropped, then
        //     the consumer calls `next()` on dangling data.
        //
        // (c) Why this is acceptable: JavaScript's single-threaded event loop
        //     guarantees that `EdgeVec` cannot be garbage-collected or dropped
        //     *during* a synchronous iteration step. The GC only runs between
        //     microtask boundaries, so as long as the user does not `free()` the
        //     `EdgeVec` while iterating, the reference remains valid. The doc comment
        //     on `save_stream` explicitly states this contract.
        //
        // If wasm-bindgen gains lifetime support in the future, this transmute
        // should be removed in favor of a safe borrowed return.
        #[allow(unsafe_code)]
        let static_iter = unsafe { std::mem::transmute::<ChunkIter<'_>, ChunkIter<'static>>(iter) };

        Ok(PersistenceIterator {
            iter: static_iter,
            liveness: self.liveness.clone(),
        })
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
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw_mut()?;
        let id = crate::hnsw::VectorId(u64::from(vector_id));
        index
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
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw()?;
        let id = crate::hnsw::VectorId(u64::from(vector_id));
        index
            .is_deleted(id)
            .map_err(|e| JsValue::from_str(&format!("is_deleted failed: {e}")))
    }

    /// Get the count of deleted (tombstoned) vectors.
    ///
    /// # Returns
    ///
    /// The number of vectors that have been soft-deleted but not yet compacted.
    #[wasm_bindgen(js_name = deletedCount)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn deleted_count(&self) -> Result<u32, JsValue> {
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw()?;
        Ok(index.deleted_count() as u32)
    }

    /// Get the count of live (non-deleted) vectors.
    ///
    /// # Returns
    ///
    /// The number of vectors that are currently searchable.
    #[wasm_bindgen(js_name = liveCount)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn live_count(&self) -> Result<u32, JsValue> {
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw()?;
        Ok(index.live_count() as u32)
    }

    /// Get the ratio of deleted to total vectors.
    ///
    /// # Returns
    ///
    /// A value between 0.0 and 1.0 representing the tombstone ratio.
    /// 0.0 means no deletions, 1.0 means all vectors deleted.
    #[wasm_bindgen(js_name = tombstoneRatio)]
    pub fn tombstone_ratio(&self) -> Result<f64, JsValue> {
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw()?;
        Ok(index.tombstone_ratio())
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
    pub fn needs_compaction(&self) -> Result<bool, JsValue> {
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw()?;
        Ok(index.needs_compaction())
    }

    /// Get the current compaction threshold.
    ///
    /// # Returns
    ///
    /// The threshold ratio (0.0 to 1.0) above which `needsCompaction()` returns true.
    /// Default is 0.3 (30%).
    #[wasm_bindgen(js_name = compactionThreshold)]
    pub fn compaction_threshold(&self) -> Result<f64, JsValue> {
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw()?;
        Ok(index.compaction_threshold())
    }

    /// Set the compaction threshold.
    ///
    /// # Arguments
    ///
    /// * `ratio` - The new threshold (clamped to 0.01 - 0.99).
    #[wasm_bindgen(js_name = setCompactionThreshold)]
    pub fn set_compaction_threshold(&mut self, ratio: f64) -> Result<(), JsValue> {
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw_mut()?;
        index.set_compaction_threshold(ratio);
        Ok(())
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
    pub fn compaction_warning(&self) -> Result<Option<String>, JsValue> {
        // HNSW-only: Soft delete is not supported for Flat index
        let (index, _storage) = self.inner.as_hnsw()?;
        Ok(index.compaction_warning())
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
        let (index, storage) = self.inner.as_hnsw_mut()?;

        let (new_index, new_storage, result) = index
            .compact(storage)
            .map_err(|e| JsValue::from_str(&format!("compact failed: {e}")))?;

        // Replace internal state with compacted versions
        self.inner = IndexVariant::Hnsw {
            index: Box::new(new_index),
            storage: new_storage,
        };

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
        let (index, _storage) = self.inner.as_hnsw_mut()?;

        // Convert Uint32Array to Vec<VectorId>
        let id_vec: Vec<u32> = ids.to_vec();
        let vec_ids: Vec<crate::hnsw::VectorId> = id_vec
            .iter()
            .map(|&id| crate::hnsw::VectorId(u64::from(id)))
            .collect();

        // Call core batch delete
        let result = index.soft_delete_batch(&vec_ids);

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
        let (index, _storage) = self.inner.as_hnsw_mut()?;

        // Convert Float64Array to Vec<VectorId>
        // Safe for IDs < 2^53 (Number.MAX_SAFE_INTEGER)
        let id_vec: Vec<f64> = ids.to_vec();
        let vec_ids: Vec<crate::hnsw::VectorId> = id_vec
            .iter()
            .map(|&id| crate::hnsw::VectorId(id as u64))
            .collect();

        // Call core batch delete
        let result = index.soft_delete_batch(&vec_ids);

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
        let dimensions = self.inner.dimensions();

        // Validate vector dimension
        let len = vector.length();
        if len != dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: dimensions as usize,
                actual: len as usize,
            })
            .into());
        }

        let vec = vector.to_vec();

        // C-SRC-3: NaN/Infinity check runs in ALL builds (not just debug).
        if vec.iter().any(|v| !v.is_finite()) {
            return Err(
                EdgeVecError::Validation("Vector contains non-finite values".to_string()).into(),
            );
        }

        // Parse JavaScript object into HashMap<String, MetadataValue>
        let metadata_map = parse_js_metadata_object(&metadata_js)?;

        // Pre-validate ALL metadata BEFORE vector insert (atomicity guarantee)
        if metadata_map.len() > MAX_KEYS_PER_VECTOR {
            return Err(EdgeVecError::Validation(format!(
                "Too many metadata keys: {} (max {})",
                metadata_map.len(),
                MAX_KEYS_PER_VECTOR
            ))
            .into());
        }

        // Validate each key-value pair upfront
        for (key, value) in &metadata_map {
            validate_key(key).map_err(|e| {
                EdgeVecError::Validation(format!("Invalid metadata key '{}': {}", key, e))
            })?;
            validate_value(value).map_err(|e| {
                EdgeVecError::Validation(format!("Invalid metadata value for '{}': {}", key, e))
            })?;
        }

        // NOW insert vector (metadata guaranteed not to fail after pre-validation)
        let (index, storage) = self.inner.as_hnsw_mut()?;
        let id = index.insert(&vec, storage).map_err(EdgeVecError::from)?;

        // Store pre-validated metadata
        #[allow(clippy::cast_possible_truncation)]
        let metadata_id = id.0 as u32;
        for (key, value) in metadata_map {
            self.metadata
                .insert(metadata_id, &key, value)
                .map_err(|e| EdgeVecError::Validation(format!("Metadata insert failed: {e}")))?;
        }

        // Track memory allocation for memory pressure monitoring
        track_vector_insert(dimensions);

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

        let (index, storage) = self.inner.as_hnsw()?;

        // Validate query dimensions
        let len = query.length();
        if len != index.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: index.config.dimensions as usize,
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
        let metadata_adapter = EdgeVecMetadataAdapter::new(&self.metadata, index.len());

        // Execute filtered search with auto strategy
        let mut searcher = FilteredSearcher::new(index, storage, &metadata_adapter);
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
        metadata::metadata_to_js_object(&self.metadata, id)
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

        let (index, storage) = self.inner.as_hnsw()?;

        // Validate query dimensions
        let len = query.length();
        if len != index.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: index.config.dimensions as usize,
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
        let results = index
            .search_bq(&query_vec, k, storage)
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

        let (index, storage) = self.inner.as_hnsw()?;

        // Validate query dimensions
        let len = query.length();
        if len != index.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: index.config.dimensions as usize,
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
        let results = index
            .search_bq_rescored(&query_vec, k, rescore_factor, storage)
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

        let (index, storage) = self.inner.as_hnsw()?;

        // Validate query dimensions
        let len = query.length();
        if len != index.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: index.config.dimensions as usize,
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
        let use_bq = opts.use_bq && index.bq_storage.is_some();
        let rescore_factor = opts.rescore_factor.max(1);

        // Execute appropriate search based on options
        let results: Vec<(crate::hnsw::VectorId, f32)> = if use_bq {
            if let Some(ref filter_str) = opts.filter {
                // BQ + filter + rescore: Use filtered search with BQ candidates
                let filter_expr =
                    parse(filter_str).map_err(|e| filter::filter_error_to_jsvalue(&e))?;

                // Get BQ candidates with overfetch
                let overfetch_k = opts.k.saturating_mul(rescore_factor);
                let bq_candidates = index
                    .search_bq(&query_vec, overfetch_k, storage)
                    .map_err(EdgeVecError::from)?;

                // Filter candidates using metadata
                let empty_map = std::collections::HashMap::new();
                let mut filtered: Vec<_> = bq_candidates
                    .into_iter()
                    .filter(|(vid, _)| {
                        let metadata = self.metadata.get_all(vid.0 as u32).unwrap_or(&empty_map);
                        crate::filter::evaluate(&filter_expr, metadata).unwrap_or(false)
                    })
                    .take(opts.k)
                    .collect();

                // Rescore filtered candidates with F32 if we have enough
                if !filtered.is_empty() {
                    use super::hnsw::rescore::rescore_top_k;
                    let rescored =
                        rescore_top_k(&filtered, &query_vec, storage, opts.k.min(filtered.len()));
                    filtered = rescored
                        .into_iter()
                        .map(|(id, dist)| (id, 1.0 / (1.0 + dist)))
                        .collect();
                }

                filtered
            } else {
                // BQ only (no filter)
                index
                    .search_bq_rescored(&query_vec, opts.k, rescore_factor, storage)
                    .map_err(EdgeVecError::from)?
            }
        } else if let Some(ref filter_str) = opts.filter {
            // F32 + filter (no BQ)
            let filter_expr = parse(filter_str).map_err(|e| filter::filter_error_to_jsvalue(&e))?;
            let metadata_adapter = EdgeVecMetadataAdapter::new(&self.metadata, index.len());
            let mut searcher = FilteredSearcher::new(index, storage, &metadata_adapter);
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
            let search_results = index
                .search(&query_vec, opts.k, storage)
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
        match &self.inner {
            IndexVariant::Hnsw { index, .. } => index.bq_storage.is_some(),
            IndexVariant::Flat { .. } => false, // Flat index doesn't support BQ
        }
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
        let (index, storage) = self.inner.as_hnsw_mut()?;
        index
            .enable_bq(storage)
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

        let (index, storage) = self.inner.as_hnsw()?;

        // Validate query dimensions
        let len = query.length();
        if len != index.config.dimensions {
            return Err(EdgeVecError::Graph(GraphError::DimensionMismatch {
                expected: index.config.dimensions as usize,
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
        let metadata_adapter = EdgeVecMetadataAdapter::new(&self.metadata, index.len());

        // Execute filtered search
        let mut searcher = FilteredSearcher::new(index, storage, &metadata_adapter);
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
                            self.metadata
                                .get_all(id)
                                .and_then(|m| serde_json::to_value(m).ok())
                        } else {
                            None
                        },
                        vector: if include_vectors {
                            Some(storage.get_vector(r.vector_id).to_vec())
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

        // needs_compaction only applies to HNSW indexes (Flat has no tombstones)
        let needs_compaction = match &self.inner {
            IndexVariant::Hnsw { index, .. } => index.needs_compaction(),
            IndexVariant::Flat { .. } => false,
        };

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

    /// Get approximate memory usage in bytes.
    ///
    /// Returns the total memory used by the index, including:
    /// - Vector storage (binary vectors)
    /// - HNSW graph structure (nodes and neighbor lists)
    /// - Internal metadata
    ///
    /// # Returns
    ///
    /// Total bytes used by the index.
    ///
    /// # Example
    ///
    /// ```javascript
    /// const bytes = index.memoryUsage();
    /// console.log(`Index using ${(bytes / 1024 / 1024).toFixed(2)} MB`);
    /// ```
    #[wasm_bindgen(js_name = "memoryUsage")]
    pub fn memory_usage(&self) -> usize {
        // Dispatch to IndexVariant which handles both Flat and HNSW
        self.inner.memory_usage()
    }

    /// Get estimated serialized size in bytes.
    ///
    /// Returns an estimate of the size when saved to disk.
    /// For Flat indexes, this is just the header + vector data.
    /// For HNSW indexes, includes graph overhead.
    #[wasm_bindgen(js_name = "serializedSize")]
    pub fn serialized_size(&self) -> usize {
        self.inner.serialized_size()
    }

    // =========================================================================
    // SPARSE VECTOR & HYBRID SEARCH API (v0.9.0 — Week 39 RFC-007)
    // =========================================================================

    /// Initialize sparse storage for hybrid search.
    ///
    /// Must be called before using sparse or hybrid search functions.
    /// Sparse storage is lazily initialized to minimize memory footprint
    /// for users who don't need hybrid search.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const db = new EdgeVec(config);
    /// db.initSparseStorage();  // Enable hybrid search
    ///
    /// // Now sparse/hybrid methods are available
    /// const id = db.insertSparse(indices, values, 10000);
    /// ```
    #[cfg(feature = "sparse")]
    #[wasm_bindgen(js_name = "initSparseStorage")]
    pub fn init_sparse_storage(&mut self) {
        if self.sparse_storage.is_none() {
            self.sparse_storage = Some(SparseStorage::new());
        }
    }

    /// Check if sparse storage is initialized.
    ///
    /// # Returns
    ///
    /// `true` if sparse storage is ready for use, `false` otherwise.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// if (!db.hasSparseStorage()) {
    ///     db.initSparseStorage();
    /// }
    /// ```
    #[cfg(feature = "sparse")]
    #[wasm_bindgen(js_name = "hasSparseStorage")]
    #[must_use]
    pub fn has_sparse_storage(&self) -> bool {
        self.sparse_storage.is_some()
    }

    /// Get the number of sparse vectors stored.
    ///
    /// # Returns
    ///
    /// Number of sparse vectors, or 0 if sparse storage is not initialized.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// console.log(`Sparse vectors: ${db.sparseCount()}`);
    /// ```
    #[cfg(feature = "sparse")]
    #[wasm_bindgen(js_name = "sparseCount")]
    #[must_use]
    pub fn sparse_count(&self) -> usize {
        self.sparse_storage.as_ref().map_or(0, SparseStorage::len)
    }

    /// Insert a sparse vector (e.g., BM25 scores).
    ///
    /// # Arguments
    ///
    /// * `indices` - Uint32Array of sparse indices (must be sorted ascending)
    /// * `values` - Float32Array of sparse values (same length as indices)
    /// * `dim` - Dimension of the sparse space (vocabulary size)
    ///
    /// # Returns
    ///
    /// The assigned sparse vector ID as a number (f64).
    ///
    /// **Note:** JavaScript numbers have integer precision up to 2^53.
    /// For most use cases (<9 quadrillion vectors), this is not a concern.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Sparse storage not initialized (call `initSparseStorage()` first)
    /// - `indices` and `values` have different lengths
    /// - `indices` are not sorted ascending
    /// - `indices` contain duplicates
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// db.initSparseStorage();
    /// const indices = new Uint32Array([0, 5, 10]);
    /// const values = new Float32Array([1.0, 2.0, 3.0]);
    /// const id = db.insertSparse(indices, values, 10000);
    /// console.log(`Inserted sparse vector with ID: ${id}`);
    /// ```
    #[cfg(feature = "sparse")]
    #[wasm_bindgen(js_name = "insertSparse")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::cast_precision_loss)]
    pub fn insert_sparse(
        &mut self,
        indices: Uint32Array,
        values: Float32Array,
        dim: u32,
    ) -> Result<f64, JsValue> {
        // Validate inputs
        if indices.length() != values.length() {
            return Err(JsValue::from_str(
                "indices and values must have the same length",
            ));
        }

        // Convert TypedArrays to Rust types
        let indices_vec: Vec<u32> = indices.to_vec();
        let values_vec: Vec<f32> = values.to_vec();

        // Create sparse vector (validates sorted, no duplicates, etc.)
        let vector = SparseVector::new(indices_vec, values_vec, dim)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Get or create sparse storage
        let sparse_storage = self.sparse_storage.get_or_insert_with(SparseStorage::new);

        // Insert
        let id = sparse_storage
            .insert(&vector)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Return as f64 (JavaScript number) - precision loss acceptable for IDs < 2^53
        Ok(id.as_u64() as f64)
    }

    /// Search sparse vectors by query.
    ///
    /// # Arguments
    ///
    /// * `indices` - Uint32Array of sparse query indices (sorted ascending)
    /// * `values` - Float32Array of sparse query values (same length as indices)
    /// * `dim` - Dimension of the sparse space (vocabulary size)
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// JSON string: `[{ "id": number, "score": number }, ...]`
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Sparse storage not initialized
    /// - `indices` and `values` have different lengths
    /// - `indices` are not sorted ascending
    /// - `k` is 0
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const indices = new Uint32Array([0, 5, 10]);
    /// const values = new Float32Array([1.0, 2.0, 3.0]);
    /// const resultsJson = db.searchSparse(indices, values, 10000, 10);
    /// const results = JSON.parse(resultsJson);
    /// for (const r of results) {
    ///     console.log(`ID: ${r.id}, Score: ${r.score}`);
    /// }
    /// ```
    #[cfg(feature = "sparse")]
    #[wasm_bindgen(js_name = "searchSparse")]
    #[allow(clippy::needless_pass_by_value)]
    pub fn search_sparse(
        &self,
        indices: Uint32Array,
        values: Float32Array,
        dim: u32,
        k: usize,
    ) -> Result<String, JsValue> {
        // Validate inputs
        if indices.length() != values.length() {
            return Err(JsValue::from_str(
                "indices and values must have the same length",
            ));
        }

        if k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }

        // Convert TypedArrays to Rust types
        let indices_vec: Vec<u32> = indices.to_vec();
        let values_vec: Vec<f32> = values.to_vec();

        // Create sparse vector (validates sorted, no duplicates, etc.)
        let query = SparseVector::new(indices_vec, values_vec, dim)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Get sparse storage (check if it exists)
        let sparse_storage = self.sparse_storage.as_ref().ok_or_else(|| {
            JsValue::from_str("Sparse storage not initialized. Call initSparseStorage() first.")
        })?;

        // Execute search
        let searcher = SparseSearcher::new(sparse_storage);
        let results = searcher.search(&query, k);

        // Convert to JSON
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id.as_u64(),
                    "score": r.score
                })
            })
            .collect();

        serde_json::to_string(&json_results).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Perform hybrid search combining dense and sparse retrieval.
    ///
    /// # Arguments
    ///
    /// * `dense_query` - Float32Array dense embedding vector
    /// * `sparse_indices` - Uint32Array sparse query indices (sorted)
    /// * `sparse_values` - Float32Array sparse query values
    /// * `sparse_dim` - Dimension of sparse space (vocabulary size)
    /// * `options_json` - JSON configuration string
    ///
    /// # Options JSON Schema
    ///
    /// ```json
    /// {
    ///   "dense_k": 20,      // Results from dense search (default: 20)
    ///   "sparse_k": 20,     // Results from sparse search (default: 20)
    ///   "k": 10,            // Final results to return (required)
    ///   "fusion": "rrf"     // or { "type": "linear", "alpha": 0.7 }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// JSON string:
    /// ```json
    /// [
    ///   {
    ///     "id": 42,
    ///     "score": 0.032,
    ///     "dense_rank": 1,
    ///     "dense_score": 0.95,
    ///     "sparse_rank": 3,
    ///     "sparse_score": 4.2
    ///   }
    /// ]
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Sparse storage not initialized
    /// - Dense query dimensions mismatch index
    /// - Sparse indices/values length mismatch
    /// - Invalid options JSON
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const denseQuery = new Float32Array([0.1, 0.2, ...]);
    /// const sparseIndices = new Uint32Array([0, 5, 10]);
    /// const sparseValues = new Float32Array([1.0, 2.0, 3.0]);
    ///
    /// const results = JSON.parse(db.hybridSearch(
    ///     denseQuery,
    ///     sparseIndices,
    ///     sparseValues,
    ///     10000,
    ///     JSON.stringify({ k: 10, fusion: 'rrf' })
    /// ));
    /// ```
    #[cfg(feature = "sparse")]
    #[wasm_bindgen(js_name = "hybridSearch")]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::too_many_arguments)]
    pub fn hybrid_search(
        &self,
        dense_query: Float32Array,
        sparse_indices: Uint32Array,
        sparse_values: Float32Array,
        sparse_dim: u32,
        options_json: &str,
    ) -> Result<String, JsValue> {
        // Hybrid search requires HNSW index (dense vectors)
        let (hnsw_index, storage) = match &self.inner {
            IndexVariant::Hnsw { index, storage } => (index.as_ref(), storage),
            IndexVariant::Flat { .. } => {
                return Err(JsValue::from_str(
                    "Hybrid search is only supported for HNSW indexes (dense vectors). \
                     Flat/binary indexes do not support hybrid search.",
                ));
            }
        };

        // Parse options
        let options: SparseHybridOptions = serde_json::from_str(options_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid options JSON: {e}")))?;

        // Validate dense query dimensions
        let expected_dims = hnsw_index.config.dimensions;
        if dense_query.length() != expected_dims {
            return Err(JsValue::from_str(&format!(
                "Dense query dimension mismatch: expected {}, got {}",
                expected_dims,
                dense_query.length()
            )));
        }

        // Validate sparse inputs
        if sparse_indices.length() != sparse_values.length() {
            return Err(JsValue::from_str(
                "sparse_indices and sparse_values must have the same length",
            ));
        }

        // Convert inputs
        let dense_vec: Vec<f32> = dense_query.to_vec();
        let sparse_indices_vec: Vec<u32> = sparse_indices.to_vec();
        let sparse_values_vec: Vec<f32> = sparse_values.to_vec();

        // Validate dense query for non-finite values
        if dense_vec.iter().any(|v| !v.is_finite()) {
            return Err(JsValue::from_str("Dense query contains non-finite values"));
        }

        // Create sparse query
        let sparse_query = SparseVector::new(sparse_indices_vec, sparse_values_vec, sparse_dim)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Get sparse storage
        let sparse_storage = self.sparse_storage.as_ref().ok_or_else(|| {
            JsValue::from_str("Sparse storage not initialized. Call initSparseStorage() first.")
        })?;

        // Build config
        let fusion = match &options.fusion {
            HybridFusionOption::Rrf => FusionMethod::rrf(),
            HybridFusionOption::Linear { alpha, .. } => {
                FusionMethod::linear(*alpha).map_err(|e| JsValue::from_str(&e))?
            }
        };

        let config = HybridSearchConfig::new(
            options.dense_k.unwrap_or(20),
            options.sparse_k.unwrap_or(20),
            options.k,
            fusion,
        );

        // Execute hybrid search
        let searcher = HybridSearcher::new(hnsw_index, storage, sparse_storage);

        let results = searcher
            .search(&dense_vec, &sparse_query, &config)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Convert to JSON
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let mut obj = serde_json::json!({
                    "id": r.id.0,
                    "score": r.score
                });

                if let Some(rank) = r.dense_rank {
                    obj["dense_rank"] = serde_json::json!(rank);
                }
                if let Some(score) = r.dense_score {
                    obj["dense_score"] = serde_json::json!(score);
                }
                if let Some(rank) = r.sparse_rank {
                    obj["sparse_rank"] = serde_json::json!(rank);
                }
                if let Some(score) = r.sparse_score {
                    obj["sparse_score"] = serde_json::json!(score);
                }

                obj
            })
            .collect();

        serde_json::to_string(&json_results).map_err(|e| JsValue::from_str(&e.to_string()))
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

// =============================================================================
// SPARSE HYBRID SEARCH OPTION TYPES (Week 39 RFC-007)
// =============================================================================

/// Options for sparse/dense hybrid search (JSON deserialization).
#[cfg(feature = "sparse")]
#[derive(Deserialize)]
struct SparseHybridOptions {
    /// Final number of results to return.
    k: usize,

    /// Number of dense results to retrieve (default: 20).
    dense_k: Option<usize>,

    /// Number of sparse results to retrieve (default: 20).
    sparse_k: Option<usize>,

    /// Fusion method.
    #[serde(default)]
    fusion: HybridFusionOption,
}

/// Fusion method option (from JSON).
#[cfg(feature = "sparse")]
#[derive(Debug, Clone, Default)]
enum HybridFusionOption {
    /// RRF fusion.
    #[default]
    Rrf,
    /// Linear combination with alpha weight.
    Linear { alpha: f32 },
}

/// Custom deserializer for fusion that handles both string ("rrf") and object ({ type: "linear", alpha: 0.7 }).
#[cfg(feature = "sparse")]
impl<'de> Deserialize<'de> for HybridFusionOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct FusionVisitor;

        impl<'de> Visitor<'de> for FusionVisitor {
            type Value = HybridFusionOption;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(r#""rrf" or { "type": "linear", "alpha": number }"#)
            }

            fn visit_str<E>(self, value: &str) -> Result<HybridFusionOption, E>
            where
                E: de::Error,
            {
                match value.to_lowercase().as_str() {
                    "rrf" => Ok(HybridFusionOption::Rrf),
                    "linear" => Err(de::Error::custom(
                        "linear fusion requires an object with alpha: { \"type\": \"linear\", \"alpha\": 0.7 }",
                    )),
                    _ => Err(de::Error::unknown_variant(value, &["rrf", "linear"])),
                }
            }

            fn visit_map<M>(self, mut map: M) -> Result<HybridFusionOption, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut fusion_type: Option<String> = None;
                let mut alpha: Option<f32> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => fusion_type = Some(map.next_value()?),
                        "alpha" => alpha = Some(map.next_value()?),
                        _ => {
                            let _: serde_json::Value = map.next_value()?;
                        }
                    }
                }

                match fusion_type.as_deref() {
                    Some("linear") => {
                        let alpha = alpha.ok_or_else(|| de::Error::missing_field("alpha"))?;
                        Ok(HybridFusionOption::Linear { alpha })
                    }
                    Some("rrf") | None => Ok(HybridFusionOption::Rrf),
                    Some(other) => Err(de::Error::unknown_variant(other, &["rrf", "linear"])),
                }
            }
        }

        deserializer.deserialize_any(FusionVisitor)
    }
}

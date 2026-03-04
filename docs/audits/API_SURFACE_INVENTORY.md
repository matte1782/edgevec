# EdgeVec API Surface Inventory

**Author:** META_ARCHITECT
**Date:** 2026-03-27
**Status:** [REVISED]
**Purpose:** Complete inventory of all public APIs for v1.0 stability assessment
**Version:** v0.9.0 (current) -> v1.0 (planned)
**Companion:** [`API_STABILITY_AUDIT.md`](API_STABILITY_AUDIT.md) — breaking change candidates and deprecation plan

---

## 1. Rust Public API

All public exports are gated through `src/lib.rs` re-exports. Items not listed below are internal.

### 1.1 Core Types

| Type | Module | Description | Stability | Notes |
|:-----|:-------|:------------|:----------|:------|
| `IndexType` | `lib` (root) | Enum: `Flat`, `Hnsw(HnswConfig)` | STABLE | Defines which index algorithm to use |
| `IndexType::flat()` | `lib` (root) | Constructor for Flat variant | STABLE | |
| `IndexType::hnsw(dimensions)` | `lib` (root) | Constructor for Hnsw variant with defaults | STABLE | |
| `IndexType::hnsw_with_config(config)` | `lib` (root) | Constructor for Hnsw with custom config | STABLE | |
| `IndexType::is_flat()` | `lib` (root) | Check if Flat | STABLE | |
| `IndexType::is_hnsw()` | `lib` (root) | Check if HNSW | STABLE | |
| `VectorStorage` | `storage` | Contiguous vector storage backend | STABLE | Core data structure |
| `VectorStorage::new(config, capacity)` | `storage` | Create storage from HnswConfig | STABLE | |
| `Metric` | `metric` | Trait for distance computation: `fn distance(a, b) -> f32` | STABLE | Implemented by L2, Cosine, DotProduct, Hamming |
| `VERSION` | `lib` (root) | Crate version `&str` constant | STABLE | `env!("CARGO_PKG_VERSION")` |
| `version()` | `lib` (root) | Returns crate version string | STABLE | |

### 1.2 HNSW Index Operations

| Function/Method | Signature | Description | Stability | Notes |
|:----------------|:----------|:------------|:----------|:------|
| `HnswIndex::new` | `(config: HnswConfig, storage: &VectorStorage) -> Result<Self, GraphError>` | Create new HNSW index | STABLE | |
| `HnswIndex::with_metadata` | `(config, storage) -> Result<Self, GraphError>` | Create with metadata store | STABLE | |
| `HnswIndex::with_bq` | `(config, storage) -> Result<Self, GraphError>` | Create with binary quantization | STABLE | |
| `HnswIndex::insert` | `(&vector, &mut storage) -> Result<VectorId, GraphError>` | Insert f32 vector | STABLE | |
| `HnswIndex::insert_binary` | `(&binary_vec, &mut storage) -> Result<VectorId, GraphError>` | Insert packed binary vector | STABLE | |
| `HnswIndex::insert_bq` | `(&vector, &mut storage) -> Result<VectorId, GraphError>` | Insert BQ-encoded vector | STABLE | |
| `HnswIndex::insert_with_bq` | `(&f32_vec, &mut storage) -> Result<VectorId, GraphError>` | Insert f32 with auto binary quantization | STABLE | |
| `HnswIndex::insert_with_metadata` | `(&vector, metadata, &mut storage) -> Result<VectorId, GraphError>` | Insert with key-value metadata | STABLE | |
| `HnswIndex::insert_with_id` | `(&vector, vector_id, &mut storage) -> Result<VectorId, GraphError>` | Insert with specific ID | STABLE | |
| `HnswIndex::batch_insert` | `(&[&[f32]], &mut storage) -> Result<Vec<VectorId>, BatchError>` | Batch insert (BatchInsertable trait) | STABLE | |
| `HnswIndex::search` | `(&query, k, &storage) -> Result<Vec<SearchResult>, GraphError>` | KNN search | STABLE | |
| `HnswIndex::search_with_context` | `(&query, k, &ctx, &storage) -> Result<Vec<SearchResult>, GraphError>` | KNN search with context | STABLE | Internal search variant |
| `HnswIndex::search_binary` | `(&query, k, &storage) -> Result<Vec<SearchResult>, GraphError>` | Binary KNN search (Hamming) | STABLE | |
| `HnswIndex::search_binary_with_context` | `(&query, k, &ctx, &storage) -> Result<Vec<SearchResult>, GraphError>` | Binary search with context | STABLE | Internal search variant |
| `HnswIndex::search_binary_with_ef` | `(&query, k, ef_search, &storage) -> Result<Vec<SearchResult>, GraphError>` | Binary search with custom ef | STABLE | |
| `HnswIndex::search_binary_with_ef_context` | `(&query, k, ef, &ctx, &storage) -> Result<Vec<SearchResult>, GraphError>` | Binary search with ef + context | STABLE | Internal search variant |
| `HnswIndex::search_bq` | `(&query, k, &storage) -> Result<Vec<(VectorId, f32)>, GraphError>` | Binary quantization search | STABLE | |
| `HnswIndex::search_bq_rescored` | `(&query, k, rescore_factor, &storage) -> Result<Vec<SearchResult>, GraphError>` | BQ search with f32 rescoring | STABLE | |
| `HnswIndex::search_bq_rescored_default` | `(&query, k, &storage) -> Result<Vec<SearchResult>, GraphError>` | BQ rescored with default factor | STABLE | |
| `HnswIndex::search_bq_high_recall` | `(&query, k, &storage) -> Result<Vec<SearchResult>, GraphError>` | BQ high recall search | STABLE | |
| `HnswIndex::search_filtered` | `(&query, k, filter, &storage) -> Result<FilteredResult, GraphError>` | Search with metadata filter | STABLE | |
| `HnswIndex::soft_delete` | `(VectorId) -> Result<bool, GraphError>` | Tombstone a vector | STABLE | |
| `HnswIndex::soft_delete_batch` | `(&[VectorId]) -> BatchDeleteResult` | Batch tombstone | STABLE | |
| `HnswIndex::soft_delete_batch_with_progress` | `(&[VectorId], progress_fn) -> BatchDeleteResult` | Batch tombstone with progress callback | STABLE | |
| `HnswIndex::is_deleted` | `(VectorId) -> Result<bool, GraphError>` | Check tombstone status | STABLE | |
| `HnswIndex::deleted_count` | `() -> usize` | Count tombstoned vectors | STABLE | |
| `HnswIndex::live_count` | `() -> usize` | Count live vectors | STABLE | |
| `HnswIndex::tombstone_ratio` | `() -> f64` | Ratio of deleted/total | STABLE | |
| `HnswIndex::needs_compaction` | `() -> bool` | Check if compaction recommended | STABLE | |
| `HnswIndex::compaction_threshold` | `() -> f64` | Get compaction threshold | STABLE | |
| `HnswIndex::set_compaction_threshold` | `(ratio: f64)` | Set compaction threshold | STABLE | |
| `HnswIndex::compaction_warning` | `() -> Option<String>` | Compaction warning message | STABLE | |
| `HnswIndex::compact` | `(&mut storage) -> Result<(HnswIndex, VectorStorage, CompactionResult), GraphError>` | Rebuild without tombstones | STABLE | |
| `HnswIndex::enable_bq` | `(&storage) -> Result<(), GraphError>` | Enable BQ on existing index | STABLE | |
| `HnswIndex::has_bq` | `() -> bool` | Check if BQ enabled | STABLE | |
| `HnswIndex::bq_storage` | `() -> Option<&BinaryVectorStorage>` | Access BQ storage | STABLE | |
| `HnswIndex::len` | `() -> usize` | Total vectors (including deleted) | STABLE | |
| `HnswIndex::is_empty` | `() -> bool` | Check if index is empty | STABLE | |
| `HnswIndex::dimensions` | `() -> u32` | Vector dimensionality | STABLE | |
| `HnswIndex::contains_id` | `(id: u64) -> bool` | Check if vector ID exists | STABLE | |
| `HnswIndex::memory_usage` | `() -> usize` | Approximate memory in bytes | STABLE | |
| `HnswIndex::metadata` | `() -> &MetadataStore` | Access metadata store | STABLE | |
| `HnswIndex::metadata_mut` | `() -> &mut MetadataStore` | Mutable metadata access | STABLE | |
| `HnswIndex::get_random_level` | `() -> u8` | Generate random layer level | STABLE | Graph internals |
| `HnswIndex::add_node` | `(vector_id, max_layer) -> Result<NodeId, GraphError>` | Add node to graph | STABLE | Graph internals |
| `HnswIndex::set_neighbors` | `(node, layer, ids) -> Result<(), GraphError>` | Set node neighbors | STABLE | Graph internals |
| `HnswIndex::get_node` | `(id) -> Option<&HnswNode>` | Get node reference | STABLE | Graph internals |
| `HnswIndex::get_neighbors` | `(node) -> Result<Vec<NodeId>, GraphError>` | Get node neighbors | STABLE | Graph internals |
| `HnswIndex::get_neighbors_layer` | `(node, layer) -> Result<Vec<NodeId>, GraphError>` | Get neighbors at layer | STABLE | Graph internals |
| `HnswIndex::entry_point` | `() -> Option<NodeId>` | Get entry point | STABLE | Graph internals |
| `HnswIndex::set_entry_point` | `(id)` | Set entry point | STABLE | Graph internals |
| `HnswIndex::node_count` | `() -> usize` | Count graph nodes | STABLE | Graph internals |
| `HnswIndex::max_layer` | `() -> u8` | Max graph layer | STABLE | Graph internals |
| `HnswIndex::adjusted_k` | `(k) -> usize` | Adjust k for live count | STABLE | Internal utility |
| `HnswIndex::delete_in_storage` | `(id, &mut storage) -> bool` | Delete from storage | STABLE | Internal utility |
| `HnswIndex::log_stats` | `()` | Log index statistics | STABLE | Debug utility |

### 1.3 HNSW Supporting Types

| Type | Module | Description | Stability | Notes |
|:-----|:-------|:------------|:----------|:------|
| `HnswConfig` | `hnsw::config` | HNSW parameters struct (32 bytes) | STABLE | Fields: m, m0, ef_construction, ef_search, dimensions, metric, _reserved |
| `HnswConfig::new(dimensions)` | `hnsw::config` | Default config: M=12, M0=24, ef_c=100, ef_s=50 | STABLE | |
| `HnswConfig::METRIC_L2_SQUARED` | `hnsw::config` | Metric constant: 0 | STABLE | |
| `HnswConfig::METRIC_COSINE` | `hnsw::config` | Metric constant: 1 | STABLE | |
| `HnswConfig::METRIC_DOT_PRODUCT` | `hnsw::config` | Metric constant: 2 | STABLE | |
| `HnswConfig::METRIC_HAMMING` | `hnsw::config` | Metric constant: 3 | STABLE | |
| `SearchResult` | `hnsw::search` | Search result: `{ vector_id: VectorId, distance: f32 }` | STABLE | |
| `VectorId(pub u64)` | `hnsw::graph` | Newtype wrapper for vector IDs | STABLE | 1-indexed (0 = INVALID) |
| `GraphError` | `hnsw::graph` | Error enum for graph operations | STABLE | |
| `BatchDeleteResult` | `hnsw::graph` | Result struct for batch soft_delete | STABLE | Fields: deleted, already_deleted, invalid_ids, total, unique_count |
| `BatchDeleteError` | `hnsw::graph` | Error enum for batch delete | STABLE | |
| `CompactionResult` | `hnsw::graph` | Result of compact operation | STABLE | Fields: tombstones_removed, new_size, duration_ms |
| `HnswGraph` | `hnsw::mod` | Type alias for `HnswIndex` | UNSTABLE | May be removed in v1.0 (redundant alias) |
| `HnswNode` | `hnsw::graph` | Graph node struct | UNSTABLE | Candidate for `pub(crate)` in v1.0 — see [stability audit](API_STABILITY_AUDIT.md) Section 3.2 |
| `NodeId` | `hnsw::graph` | Node identifier in graph (distinct from VectorId) | STABLE | Used in public method signatures (get_neighbors, add_node) |
| `Candidate` | `hnsw::search` | Search candidate struct | UNSTABLE | Candidate for `pub(crate)` in v1.0 — see [stability audit](API_STABILITY_AUDIT.md) Section 3.2 |
| `SearchContext` | `hnsw::search` | Search context for reuse | UNSTABLE | Candidate for `pub(crate)` in v1.0 — see [stability audit](API_STABILITY_AUDIT.md) Section 3.2 |
| `Searcher` | `hnsw::search` | Generic searcher struct | UNSTABLE | Candidate for `pub(crate)` in v1.0 — see [stability audit](API_STABILITY_AUDIT.md) Section 3.2 |
| `NeighborPool` | `hnsw::neighbor` | Neighbor list pool for graph ops | UNSTABLE | Candidate for `pub(crate)` in v1.0 — see [stability audit](API_STABILITY_AUDIT.md) Section 3.2 |
| `VectorProvider` | `hnsw::graph` | Trait for vector data access | STABLE | |
| `BinaryVectorStorage` | `hnsw::bq` | Storage for binary-quantized vectors | STABLE | Returned by `bq_storage()` |

### 1.4 Flat Index (F32)

| Type/Function | Module | Description | Stability | Notes |
|:-------------|:-------|:------------|:----------|:------|
| `FlatIndex` | `index::flat` | Brute-force f32 index with optional SQ8 quantization | STABLE | |
| `FlatIndex::new(config)` | `index::flat` | Create from FlatIndexConfig | STABLE | |
| `FlatIndex::insert(&[f32])` | `index::flat` | Insert f32 vector -> `Result<u64>` | STABLE | |
| `FlatIndex::insert_batch(&[&[f32]])` | `index::flat` | Batch insert -> `Result<Vec<u64>>` | STABLE | |
| `FlatIndex::get(u64)` | `index::flat` | Retrieve vector by ID | STABLE | |
| `FlatIndex::delete(u64)` | `index::flat` | Soft delete vector | STABLE | |
| `FlatIndex::compact()` | `index::flat` | Reclaim deleted space | STABLE | |
| `FlatIndex::enable_quantization()` | `index::flat` | Enable SQ8 quantization | STABLE | |
| `FlatIndex::disable_quantization()` | `index::flat` | Disable SQ8 quantization | STABLE | |
| `FlatIndexConfig` | `index::flat` | Configuration struct | STABLE | Fields: dimensions, metric, capacity, cleanup_threshold |
| `FlatIndexConfig::new(u32)` | `index::flat` | Create with defaults | STABLE | |
| `FlatIndexConfig::with_metric(DistanceMetric)` | `index::flat` | Set distance metric | STABLE | |
| `FlatIndexConfig::with_capacity(usize)` | `index::flat` | Set initial capacity | STABLE | |
| `FlatIndexError` | `index::flat` | Error enum | STABLE | |
| `FlatSearchResult` | `index::flat` | Result: `{ id: u64, distance: f32 }` | STABLE | |
| `DistanceMetric` | `index::flat` | Enum: L2, Cosine, DotProduct, InnerProduct | STABLE | |
| `FlatIndexHeader` | `index::flat` | Persistence header struct | STABLE | For serialization format |
| `FLAT_INDEX_MAGIC` | `index::flat` | Magic bytes for flat index files | STABLE | |
| `FLAT_INDEX_VERSION` | `index::flat` | Current format version | STABLE | |

### 1.5 Binary Flat Index

| Type/Function | Module | Description | Stability | Notes |
|:-------------|:-------|:------------|:----------|:------|
| `BinaryFlatIndex` | `flat` | Brute-force binary (Hamming) index | STABLE | O(1) insert, O(n) search, 100% recall |
| `BinaryFlatIndex::new(dimensions)` | `flat` | Create: dimensions = bit count | STABLE | |
| `BinaryFlatIndex::with_capacity(dimensions, capacity)` | `flat` | Create with pre-allocated capacity | STABLE | |
| `BinaryFlatIndex::insert(&[u8])` | `flat` | Insert packed binary vector -> `Result<VectorId>` | STABLE | |
| `BinaryFlatIndex::search(&[u8], k)` | `flat` | KNN by Hamming distance -> `Result<Vec<BinaryFlatSearchResult>>` | STABLE | |
| `BinaryFlatIndex::get(VectorId)` | `flat` | Retrieve vector by ID | STABLE | |
| `BinaryFlatIndex::len()` | `flat` | Vector count | STABLE | |
| `BinaryFlatIndex::is_empty()` | `flat` | Check if empty | STABLE | |
| `BinaryFlatIndex::dimensions()` | `flat` | Bit dimensions | STABLE | |
| `BinaryFlatIndex::bytes_per_vector()` | `flat` | Byte size per vector | STABLE | |
| `BinaryFlatIndex::memory_usage()` | `flat` | Approximate memory | STABLE | |
| `BinaryFlatIndex::serialized_size()` | `flat` | Serialized size | STABLE | |
| `BinaryFlatIndex::clear()` | `flat` | Remove all vectors | STABLE | |
| `BinaryFlatIndex::shrink_to_fit()` | `flat` | Release excess memory | STABLE | |
| `BinaryFlatIndexError` | `flat` | Error enum | STABLE | |
| `BinaryFlatSearchResult` | `flat` | Result: `{ id: VectorId, distance: f32 }` | STABLE | |

### 1.6 Quantization

| Type/Function | Module | Description | Stability | Notes |
|:-------------|:-------|:------------|:----------|:------|
| `ScalarQuantizer` | `quantization::scalar` | SQ8 quantizer (f32 -> u8) | STABLE | 4x memory reduction |
| `ScalarQuantizer::new(config)` | `quantization::scalar` | Create from config | STABLE | |
| `ScalarQuantizer::train(&[&[f32]])` | `quantization::scalar` | Train on vector samples | STABLE | |
| `ScalarQuantizer::quantize(&[f32])` | `quantization::scalar` | Quantize a vector -> `Vec<u8>` | STABLE | |
| `ScalarQuantizer::dequantize(&[u8])` | `quantization::scalar` | Dequantize back -> `Vec<f32>` | STABLE | |
| `QuantizerConfig` | `quantization::scalar` | Config: min_val, max_val | STABLE | |
| `BinaryQuantizer` | `quantization::binary` | 1-bit quantizer (f32 -> packed bits) | STABLE | 32x memory reduction |
| `QuantizedVector` | `quantization::binary` | Packed binary vector with hamming/similarity methods | STABLE | |

### 1.7 Persistence

| Function/Type | Module | Description | Stability | Notes |
|:-------------|:-------|:------------|:----------|:------|
| `write_snapshot(index, storage, backend)` | `persistence::snapshot` | Serialize index+storage to backend | STABLE | |
| `read_snapshot(backend)` | `persistence::snapshot` | Deserialize index+storage from backend | STABLE | |
| `MemoryBackend` | `persistence::storage` | In-memory storage backend | STABLE | |
| `StorageBackend` | `persistence::storage` | Trait for storage backends | STABLE | |
| `ChunkedWriter` | `persistence::chunking` | Trait for chunked serialization | STABLE | Used by WASM streaming persistence |
| `write_empty_index(config)` | `persistence::writer` | Create empty index bytes | STABLE | |
| `read_index_header(data)` | `persistence::reader` | Parse index header | STABLE | |
| `read_file_header(data)` | `persistence::reader` | Parse file header | STABLE | |
| `PersistenceError` | `persistence` | Error enum | STABLE | |
| `FileHeader` | `persistence::header` | File header struct | STABLE | |
| `MAGIC` | `persistence::header` | Magic bytes: `b"EVEC"` | STABLE | |
| `VERSION_MAJOR` | `persistence::header` | Major version: 0 | UNSTABLE | Will change to 1 at v1.0 |
| `VERSION_MINOR` / `VERSION_MINOR_MIN` | `persistence::header` | Minor version bounds | UNSTABLE | |

### 1.8 Filter API

| Type/Function | Module | Description | Stability | Notes |
|:-------------|:-------|:------------|:----------|:------|
| `filter::parse(query)` | `filter` | Parse filter expression string | STABLE | SQL-like DSL |
| `filter::evaluate(expr, metadata)` | `filter` | Evaluate filter against metadata map | STABLE | |
| `filter::FilterExpression` | `filter` | Compiled filter AST | STABLE | |
| `filter::FilterStrategy` | `filter` | Enum: Auto, PreFilter, PostFilter, Hybrid | STABLE | |
| `filter::FilteredSearcher` | `filter` | Combines HNSW search with filter evaluation | STABLE | |
| `filter::MetadataStore` | `filter` | Trait that filter system uses for metadata access | STABLE | |

### 1.9 Error Types

| Type | Module | Description | Stability | Notes |
|:-----|:-------|:------------|:----------|:------|
| `EdgeVecError` | `error` | Top-level error enum | STABLE | Variants: Graph, Persistence, Validation, Batch |
| `BatchError` | `error` | Batch operation error enum | STABLE | |

### 1.10 Batch API

| Type/Trait | Module | Description | Stability | Notes |
|:-----------|:-------|:------------|:----------|:------|
| `BatchInsertable` | `batch` | Trait for batch insert operations | STABLE | |

### 1.11 SIMD

| Type/Function | Module | Description | Stability | Notes |
|:-------------|:-------|:------------|:----------|:------|
| `SimdBackend` | `simd` | Enum: Scalar, Sse2, Avx2, Neon, WasmSimd128 | STABLE | Runtime detection |
| `SimdCapabilities` | `simd::detect` | Detected SIMD features struct | STABLE | |
| `capabilities()` | `simd::detect` | Get detected capabilities (cached) | STABLE | |
| `detect_neon()` | `simd` | Check NEON support | STABLE | |
| `select_backend()` | `simd` | Select best SIMD backend | STABLE | |
| `warn_if_suboptimal()` | `simd` | Log warning if not using best SIMD | STABLE | |

### 1.12 Sparse Vectors (feature-gated: `sparse`)

| Type/Function | Module | Description | Stability | Notes |
|:-------------|:-------|:------------|:----------|:------|
| `SparseVector` | `sparse` | Sparse vector: indices + values + dim | EXPERIMENTAL | Requires `sparse` feature |
| `SparseError` | `sparse` | Error enum for sparse operations | EXPERIMENTAL | Requires `sparse` feature |

---

## 2. WASM API

All WASM exports are in `src/wasm/mod.rs` and submodules. Compiled only for `wasm32` targets.

### 2.1 Top-Level Functions

| JS Name | Rust Function | Parameters | Returns | Description | Stability |
|:--------|:-------------|:-----------|:--------|:------------|:----------|
| `init_logging()` | `init_logging` | none | `void` | Initialize console panic hook + logging | STABLE |
| `getSimdBackend()` | `get_simd_backend` | none | `string` | Returns "wasm_simd128", "avx2", or "scalar" | STABLE |
| `benchmarkHamming(bytes, iterations)` | `benchmark_hamming` | `usize, usize` | `f64` | Microbenchmark: us/iteration for Hamming | EXPERIMENTAL |
| `benchmarkHammingBatch(vectors, query, iterations)` | `benchmark_hamming_batch` | `Array, Uint8Array, usize` | `string` (JSON) | Batch benchmark comparing SIMD impls | EXPERIMENTAL |

### 2.2 Enums

| JS Name | Variants | Description | Stability |
|:--------|:---------|:------------|:----------|
| `VectorType` | `Float32 = 0`, `Binary = 1` | Storage type selector | STABLE |
| `MetricType` | `L2 = 0`, `Cosine = 1`, `Dot = 2`, `Hamming = 3` | Distance metric selector | STABLE |
| `IndexType` (JS: `JsIndexType`) | `Flat = 0`, `Hnsw = 1` | Index algorithm selector | STABLE |

### 2.3 EdgeVecConfig Class

| JS Name | Type | Parameters | Returns | Description | Stability |
|:--------|:-----|:-----------|:--------|:------------|:----------|
| `new EdgeVecConfig(dimensions)` | constructor | `u32` | `EdgeVecConfig` | Create config | STABLE |
| `.dimensions` | field (pub) | | `u32` | Vector dimensionality | STABLE |
| `.m = value` | setter | `u32` | | Set M parameter | STABLE |
| `.m0 = value` | setter | `u32` | | Set M0 parameter | STABLE |
| `.ef_construction = value` | setter | `u32` | | Set ef_construction | STABLE |
| `.ef_search = value` | setter | `u32` | | Set ef_search | STABLE |
| `.metric = value` | setter | `string` | | Set metric: "l2","cosine","dot","hamming" | STABLE |
| `.setMetricType(type)` | method | `MetricType` | | Set metric via enum | STABLE |
| `.vectorType` | getter/setter | `VectorType` | `VectorType?` | Get/set vector storage type | STABLE |
| `.indexType` | getter/setter | `JsIndexType` | `JsIndexType` | Get/set index algorithm | STABLE |
| `.isFlat()` | method | | `bool` | Check if Flat index | STABLE |
| `.isHnsw()` | method | | `bool` | Check if HNSW index | STABLE |

### 2.4 EdgeVec Class (Main Database Handle)

#### 2.4.1 Lifecycle

| JS Name | Parameters | Returns | Description | Stability |
|:--------|:-----------|:--------|:------------|:----------|
| `new EdgeVec(config)` | `EdgeVecConfig` | `EdgeVec` | Create database | STABLE |
| `EdgeVec.load(name)` | `string` | `Promise<EdgeVec>` | Load from IndexedDB | STABLE |
| `save(name)` | `string` | `Promise<void>` | Save to IndexedDB | STABLE |
| `save_stream(chunkSize?)` | `usize?` | `PersistenceIterator` | Chunked streaming save | STABLE |

#### 2.4.2 Insert Operations

| JS Name | Parameters | Returns | Description | Stability |
|:--------|:-----------|:--------|:------------|:----------|
| `insert(vector)` | `Float32Array` | `u32` | Insert f32 vector (HNSW only) | STABLE |
| `insertBinary(vector)` | `Uint8Array` | `u32` | Insert packed binary vector | STABLE |
| `insertWithBq(vector)` | `Float32Array` | `u32` | Insert f32 with auto binary quantization | STABLE |
| `insertWithMetadata(vector, metadata)` | `Float32Array, JsValue` | `u32` | Insert with metadata object | STABLE |
| `insertBatchFlat(vectors, count)` | `Float32Array, usize` | `Uint32Array` | Legacy batch insert (flat array) | UNSTABLE | May be deprecated in favor of `insertBatch` |
| `insertBatch(vectors, config?)` | `Array, BatchInsertConfig?` | `BatchInsertResult` | New batch insert API | STABLE |
| `insertBatchWithProgress(vectors, onProgress)` | `Array, Function` | `BatchInsertResult` | Batch insert with progress callback | STABLE |

#### 2.4.3 Search Operations

| JS Name | Parameters | Returns | Description | Stability |
|:--------|:-----------|:--------|:------------|:----------|
| `search(query, k)` | `Float32Array, usize` | `Array<{id, score}>` | KNN search (HNSW f32) | STABLE |
| `searchBinary(query, k)` | `Uint8Array, usize` | `Array<{id, score}>` | Binary KNN (Hamming) | STABLE |
| `searchBinaryWithEf(query, k, efSearch)` | `Uint8Array, usize, usize` | `Array<{id, score}>` | Binary search with custom ef | STABLE |
| `searchBinaryFiltered(query, k, optionsJson)` | `Uint8Array, usize, string` | `string` (JSON) | Binary search + metadata filter | STABLE |
| `searchWithFilter(query, filter, k)` | `Float32Array, string, usize` | `Array<{id, distance}>` | Simple filtered search | STABLE |
| `searchFiltered(query, k, optionsJson)` | `Float32Array, usize, string` | `string` (JSON) | Full filtered search with diagnostics | STABLE |
| `searchBQ(query, k)` | `Float32Array, usize` | `Array<{id, distance}>` | BQ-only search (~70-85% recall) | STABLE |
| `searchBQRescored(query, k, rescoreFactor)` | `Float32Array, usize, usize` | `Array<{id, distance}>` | BQ + f32 rescoring (~95% recall) | STABLE |
| `searchHybrid(query, options)` | `Float32Array, JsValue` | `Array<{id, distance}>` | BQ + filter combined search | STABLE |

#### 2.4.4 Soft Delete & Compaction

| JS Name | Parameters | Returns | Description | Stability |
|:--------|:-----------|:--------|:------------|:----------|
| `softDelete(vectorId)` | `u32` | `bool` | Tombstone a vector | STABLE |
| `softDeleteBatch(ids)` | `Uint32Array` | `WasmBatchDeleteResult` | Batch tombstone (modern browsers) | STABLE |
| `softDeleteBatchCompat(ids)` | `Float64Array` | `WasmBatchDeleteResult` | Batch tombstone (Safari 14 compat) | STABLE |
| `isDeleted(vectorId)` | `u32` | `bool` | Check tombstone status | STABLE |
| `deletedCount()` | | `u32` | Count of tombstoned vectors | STABLE |
| `liveCount()` | | `u32` | Count of live vectors | STABLE |
| `tombstoneRatio()` | | `f64` | Deleted/total ratio | STABLE |
| `needsCompaction()` | | `bool` | Check if compaction recommended | STABLE |
| `compactionThreshold()` | | `f64` | Get threshold ratio | STABLE |
| `setCompactionThreshold(ratio)` | `f64` | `void` | Set threshold (clamped 0.01-0.99) | STABLE |
| `compactionWarning()` | | `string?` | Warning message or null | STABLE |
| `compact()` | | `WasmCompactionResult` | Rebuild without tombstones | STABLE |

#### 2.4.5 Metadata Operations

| JS Name | Parameters | Returns | Description | Stability |
|:--------|:-----------|:--------|:------------|:----------|
| `setMetadata(vectorId, key, value)` | `u32, string, JsMetadataValue` | `void` | Set single metadata key-value | STABLE |
| `getMetadata(vectorId, key)` | `u32, string` | `JsMetadataValue?` | Get single metadata value | STABLE |
| `getAllMetadata(vectorId)` | `u32` | `JsValue` (object or undefined) | Get all metadata for vector | STABLE |
| `deleteMetadata(vectorId, key)` | `u32, string` | `bool` | Delete single metadata key | STABLE |
| `deleteAllMetadata(vectorId)` | `u32` | `bool` | Delete all metadata for vector | STABLE |
| `hasMetadata(vectorId, key)` | `u32, string` | `bool` | Check if key exists | STABLE |
| `metadataKeyCount(vectorId)` | `u32` | `usize` | Count keys for vector | STABLE |
| `metadataVectorCount()` | | `usize` | Count vectors with metadata | STABLE |
| `totalMetadataCount()` | | `usize` | Total key-value pairs | STABLE |
| `getVectorMetadata(id)` | `u32` | `JsValue` | Alias for getAllMetadata | STABLE |

#### 2.4.6 Binary Quantization Control

| JS Name | Parameters | Returns | Description | Stability |
|:--------|:-----------|:--------|:------------|:----------|
| `hasBQ()` | | `bool` | Check if BQ enabled | STABLE |
| `enableBQ()` | | `void` | Enable BQ (dimensions must be divisible by 8) | STABLE |

#### 2.4.7 Memory Pressure API

| JS Name | Parameters | Returns | Description | Stability |
|:--------|:-----------|:--------|:------------|:----------|
| `getMemoryPressure()` | | `JsValue` (MemoryPressure) | Current memory state | STABLE |
| `setMemoryConfig(config)` | `JsValue` | `void` | Configure thresholds | STABLE |
| `canInsert()` | | `bool` | Check if inserts allowed | STABLE |
| `getMemoryRecommendation()` | | `JsValue` (MemoryRecommendation) | Actionable guidance | STABLE |
| `getMemoryConfig()` | | `JsValue` (MemoryConfig) | Get current config | STABLE |
| `memoryUsage()` | | `usize` | Approximate bytes used | STABLE |
| `serializedSize()` | | `usize` | Estimated save size | STABLE |

#### 2.4.8 Sparse / Hybrid Search (feature: `sparse`)

| JS Name | Parameters | Returns | Description | Stability |
|:--------|:-----------|:--------|:------------|:----------|
| `initSparseStorage()` | | `void` | Enable sparse storage | EXPERIMENTAL |
| `hasSparseStorage()` | | `bool` | Check if sparse enabled | EXPERIMENTAL |
| `sparseCount()` | | `usize` | Number of sparse vectors | EXPERIMENTAL |
| `insertSparse(indices, values, dim)` | `Uint32Array, Float32Array, u32` | `f64` (ID) | Insert sparse vector | EXPERIMENTAL |
| `searchSparse(indices, values, dim, k)` | `Uint32Array, Float32Array, u32, usize` | `string` (JSON) | Sparse search | EXPERIMENTAL |
| `hybridSearch(denseQuery, sparseIndices, sparseValues, sparseDim, optionsJson)` | `Float32Array, Uint32Array, Float32Array, u32, string` | `string` (JSON) | Dense+sparse fusion search | EXPERIMENTAL |

### 2.5 WASM Result Types

| JS Name | Fields | Description | Stability |
|:--------|:-------|:------------|:----------|
| `WasmCompactionResult` | `tombstones_removed: u32`, `new_size: u32`, `duration_ms: u32` | Compaction metrics | STABLE |
| `WasmBatchDeleteResult` | `.deleted`, `.alreadyDeleted`, `.invalidIds`, `.total`, `.uniqueCount`, `.allValid()`, `.anyDeleted()` | Batch delete metrics | STABLE |
| `BatchInsertResult` | (from `wasm::batch`) | Batch insert metrics | STABLE |
| `BatchInsertConfig` | (from `wasm::batch`) | Batch insert options | STABLE |
| `PersistenceIterator` | (from `wasm::iterator`) | Chunked save iterator | STABLE |
| `JsMetadataValue` | (from `wasm::metadata`) | Typed metadata value wrapper | STABLE |
| `MemoryPressure` | (from `wasm::memory`) | Memory state: level, usedBytes, totalBytes, usagePercent | STABLE |
| `MemoryConfig` | (from `wasm::memory`) | Thresholds + auto-compact/block settings | STABLE |
| `MemoryRecommendation` | (from `wasm::memory`) | Actionable guidance | STABLE |

---

## 3. TypeScript API (`pkg/edgevec-types.d.ts` + `pkg/edgevec-wrapper.d.ts`)

### 3.1 Core Wrapper Class (`edgevec-wrapper`)

| Export | Type | Description | Stability |
|:-------|:-----|:------------|:----------|
| `EdgeVecIndex` (class) | default export | High-level wrapper for EdgeVec WASM | STABLE |
| `EdgeVecIndex.constructor(config: IndexConfig)` | constructor | Create index | STABLE |
| `EdgeVecIndex.size` | getter | Vector count | STABLE |
| `EdgeVecIndex.dimensions` | getter | Dimensionality | STABLE |
| `EdgeVecIndex.add(vector, metadata?)` | method | Insert with optional metadata -> `number` | STABLE |
| `EdgeVecIndex.search(query, k, options?)` | method | Filtered search -> `Promise<SearchResult[]>` | STABLE |
| `EdgeVecIndex.searchFiltered(query, k, options?)` | method | Search with diagnostics -> `Promise<FilteredSearchResult>` | STABLE |
| `EdgeVecIndex.count(filter?)` | method | Count matching vectors -> `Promise<number>` | STABLE |
| `EdgeVecIndex.getMetadata(id)` | method | Get metadata -> `Metadata \| undefined` | STABLE |
| `EdgeVecIndex.setMetadata(id, key, value)` | method | Set single metadata key | STABLE |
| `EdgeVecIndex.delete(id)` | method | Soft delete -> `boolean` | STABLE |
| `EdgeVecIndex.save(name)` | method | Save to IndexedDB | STABLE |
| `EdgeVecIndex.load(name)` | static | Load from IndexedDB -> `Promise<EdgeVecIndex>` | STABLE |
| `EdgeVecIndex.initSparseStorage()` | method | Enable sparse storage | EXPERIMENTAL |
| `EdgeVecIndex.hasSparseStorage()` | method | Check sparse enabled | EXPERIMENTAL |
| `EdgeVecIndex.sparseCount()` | method | Sparse vector count | EXPERIMENTAL |
| `EdgeVecIndex.insertSparse(indices, values, dim)` | method | Insert sparse vector -> `number` | EXPERIMENTAL |
| `EdgeVecIndex.searchSparse(indices, values, dim, k)` | method | Sparse search -> `SparseSearchResult[]` | EXPERIMENTAL |
| `EdgeVecIndex.hybridSearch(dense, sparseIdx, sparseVal, dim, opts)` | method | Hybrid search -> `HybridSearchResult[]` | EXPERIMENTAL |
| `EdgeVecIndex.insertBinary(vector)` | method | Insert binary -> `number` | STABLE |
| `EdgeVecIndex.searchBinary(query, k)` | method | Binary search -> `SearchResult[]` | STABLE |
| `EdgeVecIndex.searchBinaryWithEf(query, k, ef)` | method | Binary search with custom ef | STABLE |
| `EdgeVecIndex.insertBatchFlat(vectors, count)` | method | Legacy batch insert -> `Uint32Array` | UNSTABLE |

### 3.2 TypeScript Types (`edgevec-types.d.ts`)

| Type | Fields/Shape | Description | Stability |
|:-----|:-------------|:------------|:----------|
| `MetadataValue` | `string \| number \| boolean \| string[]` | Supported metadata value types | STABLE |
| `Metadata` | `Record<string, MetadataValue>` | Metadata record | STABLE |
| `FilterExpression` | `{ _json, toString(), toJSON(), isTautology, isContradiction, complexity }` | Compiled filter | STABLE |
| `FilterValidation` | `{ valid, errors[], warnings[], filter? }` | Validation result | STABLE |
| `FilterValidationError` | `{ code, message, position?, suggestion? }` | Filter error detail | STABLE |
| `FilterValidationWarning` | `{ code, message, position? }` | Filter warning detail | STABLE |
| `SourcePosition` | `{ line, column, offset }` | Position in filter string | STABLE |
| `FilterStrategy` | `'auto' \| 'pre' \| 'post' \| 'hybrid'` | Search filter strategy | STABLE |
| `SearchOptions` | `{ filter?, strategy?, oversampleFactor?, includeMetadata?, includeVectors?, efSearch? }` | Search configuration | STABLE |
| `SearchResult` | `{ id, score, metadata?, vector? }` | Single search result | STABLE |
| `FilteredSearchResult` | `{ results[], complete, observedSelectivity, strategyUsed, vectorsEvaluated, filterTimeMs, totalTimeMs }` | Full search diagnostics | STABLE |
| `IndexConfig` | `{ dimensions, m?, efConstruction?, quantized? }` | Index construction config | STABLE |
| `FilterErrorCode` | Enum: E001-E007 (syntax), E101-E104 (type), E201-E204 (runtime), E301-E304 (limit), E401 (strategy) | Error codes | STABLE |
| `SparseVector` | `{ indices: Uint32Array, values: Float32Array, dim: number }` | Sparse vector | EXPERIMENTAL |
| `SparseSearchResult` | `{ id, score }` | Sparse search result | EXPERIMENTAL |
| `FusionMethod` | `'rrf' \| { type: 'linear', alpha: number }` | Hybrid fusion method | EXPERIMENTAL |
| `HybridSearchOptions` | `{ dense_k?, sparse_k?, k, fusion? }` | Hybrid search config | EXPERIMENTAL |
| `HybridSearchResult` | `{ id, score, dense_rank?, dense_score?, sparse_rank?, sparse_score? }` | Hybrid result | EXPERIMENTAL |

### 3.3 Filter API (`edgevec-types.d.ts`)

| Export | Type | Description | Stability |
|:-------|:-----|:------------|:----------|
| `Filter` | `FilterStatic` object | Factory for creating filter expressions | STABLE |
| `Filter.parse(query)` | method | Parse filter string -> `FilterExpression` (throws) | STABLE |
| `Filter.tryParse(query)` | method | Parse filter string -> `FilterExpression \| null` | STABLE |
| `Filter.validate(query)` | method | Validate without compiling -> `FilterValidation` | STABLE |
| `Filter.eq/ne/lt/le/gt/ge(field, value)` | methods | Comparison operators -> `FilterExpression` | STABLE |
| `Filter.between(field, low, high)` | method | Range filter | STABLE |
| `Filter.contains/startsWith/endsWith/like(field, str)` | methods | String operators | STABLE |
| `Filter.in/notIn(field, values)` | methods | Set membership | STABLE |
| `Filter.any/allOf/none(field, values)` | methods | Array operators | STABLE |
| `Filter.isNull/isNotNull(field)` | methods | Null checks | STABLE |
| `Filter.and/or/not(...)` | methods | Logical combinators | STABLE |
| `Filter.matchAll` | readonly | Matches all vectors | STABLE |
| `Filter.nothing` | readonly | Matches no vectors | STABLE |
| `FilterBuilder` | class | Fluent filter builder | STABLE |
| `FilterBuilder.where/and/or(field)` | methods | Start conditions -> `FieldCondition` | STABLE |
| `FilterBuilder.andGroup/orGroup(fn)` | methods | Grouped sub-expressions | STABLE |
| `FilterBuilder.andFilter/orFilter(expr)` | methods | Compose existing expressions | STABLE |
| `FilterBuilder.build()` | method | Compile to `FilterExpression` | STABLE |
| `FilterException` | class (extends Error) | Rich error with code, position, suggestion | STABLE |

### 3.4 Helper Functions (`edgevec-types.d.ts`)

| Function | Signature | Description | Stability |
|:---------|:----------|:------------|:----------|
| `createSparseVector(termScores, dim)` | `(Record<number, number>, number) -> SparseVector` | Create sparse from term scores | EXPERIMENTAL |
| `parseHybridResults(json)` | `(string) -> HybridSearchResult[]` | Parse raw WASM JSON | EXPERIMENTAL |
| `parseSparseResults(json)` | `(string) -> SparseSearchResult[]` | Parse raw WASM JSON | EXPERIMENTAL |
| `createHybridOptions(options)` | `(HybridSearchOptions) -> string` | Build options JSON for WASM | EXPERIMENTAL |

### 3.5 Re-exports from Wrapper

| Export | Source | Stability |
|:-------|:-------|:----------|
| `Filter` | `./filter.js` | STABLE |
| `FilterExpression` | `./filter.js` | STABLE |
| `MetadataValue` | `./filter.js` | STABLE |
| `FilterBuilder` | `./filter-builder.js` | STABLE |
| `FieldCondition` | `./filter-builder.js` | STABLE |
| `SparseSearchResult` (type) | `./edgevec-types.js` | EXPERIMENTAL |
| `HybridSearchResult` (type) | `./edgevec-types.js` | EXPERIMENTAL |
| `HybridSearchOptions` (type) | `./edgevec-types.js` | EXPERIMENTAL |

---

## 4. edgevec-langchain API

### 4.1 Exported Classes

| Class | Extends | Methods | Description | Stability |
|:------|:--------|:--------|:------------|:----------|
| `EdgeVecStore` | `SaveableVectorStore` | `addVectors`, `addDocuments`, `delete`, `similaritySearchVectorWithScore`, `save`, `load`, `fromTexts`, `fromDocuments` | LangChain VectorStore adapter | STABLE |
| `MetadataSerializationError` | `Error` | (constructor only) | Thrown on circular refs / serialization failure | STABLE |
| `EdgeVecPersistenceError` | `Error` | (constructor only) | Thrown on IndexedDB save/load failure | STABLE |
| `EdgeVecNotInitializedError` | `Error` | (constructor only) | Thrown when WASM not initialized | STABLE |

### 4.2 Exported Functions

| Function | Signature | Description | Stability |
|:---------|:----------|:------------|:----------|
| `initEdgeVec()` | `() -> Promise<void>` | Initialize WASM (idempotent, concurrent-safe) | STABLE |
| `serializeMetadata(metadata)` | `(Record<string, unknown>) -> Metadata` | LangChain metadata -> EdgeVec metadata | STABLE |
| `deserializeMetadata(metadata)` | `(Metadata) -> Record<string, unknown>` | EdgeVec metadata -> LangChain metadata | STABLE |

### 4.3 Exported Types

| Type | Fields | Description | Stability |
|:-----|:-------|:------------|:----------|
| `EdgeVecStoreConfig` | extends `IndexConfig` + `metric?: EdgeVecMetric` | Store configuration | STABLE |
| `EdgeVecMetric` | `"cosine" \| "l2" \| "dotproduct"` | Distance metric for score normalization | STABLE |

### 4.4 Re-exports

| Export | Source | Description | Stability |
|:-------|:-------|:------------|:----------|
| `Filter` | `edgevec/edgevec-wrapper.js` | Filter factory | STABLE |
| `FilterExpression` (type) | `edgevec/edgevec-wrapper.js` | Compiled filter type | STABLE |

### 4.5 EdgeVecStore Method Details

| Method | Signature | Description | Stability |
|:-------|:----------|:------------|:----------|
| `constructor(embeddings, config, _internal?)` | `(EmbeddingsInterface, EdgeVecStoreConfig, internal?)` | Create store (WASM must be init'd) | STABLE |
| `addVectors(vectors, documents, options?)` | `(number[][], DocumentInterface[], {ids?}?) -> Promise<string[]>` | Add precomputed embeddings | STABLE |
| `addDocuments(documents, options?)` | `(DocumentInterface[], {ids?}?) -> Promise<string[]>` | Embed and add documents | STABLE |
| `delete(params)` | `({ids: string[]}) -> Promise<void>` | Delete by string IDs | STABLE |
| `similaritySearchVectorWithScore(query, k, filter?)` | `(number[], number, string\|FilterExpression?) -> Promise<[Document, number][]>` | Core search method | STABLE |
| `save(directory)` | `(string) -> Promise<void>` | Persist to IndexedDB | STABLE |
| `EdgeVecStore.load(directory, embeddings)` | `(string, EmbeddingsInterface) -> Promise<EdgeVecStore>` | Restore from IndexedDB | STABLE |
| `EdgeVecStore.fromTexts(texts, metadatas, embeddings, config)` | `(string[], object[]\|object, EmbeddingsInterface, EdgeVecStoreConfig) -> Promise<EdgeVecStore>` | Factory: create from texts | STABLE |
| `EdgeVecStore.fromDocuments(docs, embeddings, config)` | `(DocumentInterface[], EmbeddingsInterface, EdgeVecStoreConfig) -> Promise<EdgeVecStore>` | Factory: create from docs | STABLE |

---

## 5. Summary Statistics

> **Methodology:** Counts are derived by automated row count of table data rows (lines starting with `| \``) in Sections 1-4. Each row counts as one API item. Compound items sharing a row (e.g., `VERSION_MINOR / VERSION_MINOR_MIN`) count as one row. Re-exports are counted at the surface where they appear.

| Surface | Total APIs | STABLE | UNSTABLE | EXPERIMENTAL |
|:--------|:----------|:-------|:---------|:-------------|
| **Rust Public API** | 161 | 151 | 8 | 2 |
| **WASM Exports** | 85 | 76 | 1 | 8 |
| **TypeScript Types** | 72 | 53 | 1 | 18 |
| **edgevec-langchain** | 20 | 20 | 0 | 0 |
| **TOTAL** | **338** | **300** | **10** | **28** |

### UNSTABLE Items (require v1.0 decision)

| Item | Surface | Reason |
|:-----|:--------|:-------|
| `HnswGraph` type alias | Rust | Redundant alias for `HnswIndex`; remove or keep for backwards compat |
| `VERSION_MAJOR` / `VERSION_MINOR` | Rust persistence | Will change at v1.0 format freeze |
| `insertBatchFlat` | WASM + TS | Legacy API superseded by `insertBatch`; deprecation candidate |

### EXPERIMENTAL Items (may be removed/changed)

| Item | Surface | Reason |
|:-----|:--------|:-------|
| `SparseVector`, `SparseError` | Rust | Feature-gated `sparse`; API shape may change |
| `initSparseStorage/hasSparseStorage/sparseCount` | WASM + TS | Sparse feature not yet battle-tested |
| `insertSparse/searchSparse/hybridSearch` | WASM + TS | Sparse search API subject to change |
| `createSparseVector/parseHybridResults/parseSparseResults/createHybridOptions` | TS helpers | Tied to sparse experimental API |
| `benchmarkHamming/benchmarkHammingBatch` | WASM | Dev-only benchmarking functions |

---

## 6. v1.0 API Freeze Recommendations

### FREEZE (no changes)
- All STABLE items above (300 APIs)
- Filter DSL syntax and error codes
- Persistence format (with version bump to 1.0)
- Score normalization formulas in edgevec-langchain

### DECIDE BEFORE FREEZE
1. **`insertBatchFlat`** -- deprecate with `#[deprecated]` annotation or remove?
2. **`HnswGraph` alias** -- remove or keep?
3. **Sparse/Hybrid APIs** -- promote to STABLE or keep behind feature gate?
4. **Benchmark functions** -- move to separate dev-only WASM build or keep?
5. **`VERSION_MAJOR`** -- bump to 1 at release

### KNOWN GAPS (not yet in public API)
- No public `compact()` on `FlatIndex` via WASM (Rust has it, WASM does not expose it)
- No `getVector(id)` to retrieve raw vector data via WASM (only via search with `includeVectors`)
- No `updateMetadata(id, metadata)` bulk update (only key-by-key via `setMetadata`)
- No iterator/cursor API for scanning all vectors

---

## META_ARCHITECT: Task Complete

Artifacts generated:
- `docs/audits/API_SURFACE_INVENTORY.md` (v1.0)

Status: PENDING_HOSTILE_REVIEW

Next: Run `/review API_SURFACE_INVENTORY.md` to validate before v1.0 freeze decision.

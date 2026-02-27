# FlatIndex API Reference

**Version:** EdgeVec v0.9.0
**Last Updated:** 2026-02-27

---

## Overview

`FlatIndex` is a brute-force exact nearest neighbor search index. It stores vectors in a contiguous row-major memory layout and performs exhaustive distance computation during search.

**Guarantees:**

- **100% recall** -- every vector is compared, producing exact results
- **O(1) insert** -- vectors are appended without graph construction
- **O(n * d) search** -- linear scan over all n vectors of dimension d
- **Low memory overhead** -- no graph structure, just vectors + deletion bitmap

**Best suited for:**

- Small datasets (under 10,000 vectors)
- Precision-critical applications requiring exact results
- Append-heavy workloads (real-time embeddings)
- Scenarios where insert latency matters more than search latency

---

## Quick Start

### Rust

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig, DistanceMetric};

// Create a flat index for 128-dimensional vectors
let config = FlatIndexConfig::new(128)
    .with_metric(DistanceMetric::Cosine)
    .with_capacity(1000);
let mut index = FlatIndex::new(config);

// Insert vectors (O(1) per insert)
let id1 = index.insert(&[0.1; 128]).unwrap();
let id2 = index.insert(&[0.2; 128]).unwrap();

// Search for 5 nearest neighbors
let results = index.search(&[0.15; 128], 5).unwrap();
for r in &results {
    println!("id={}, score={:.4}", r.id, r.score);
}
```

### JavaScript (WASM)

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();

// Create a flat index
const config = new EdgeVecConfig(128);
config.metric = "cosine";
config.indexType = "flat";

const index = new EdgeVec(config);

// Insert vectors
const v1 = new Float32Array(128).fill(0.1);
const v2 = new Float32Array(128).fill(0.2);
const id1 = index.insert(v1);
const id2 = index.insert(v2);

// Search
const query = new Float32Array(128).fill(0.15);
const results = index.search(query, 5);
console.log(results);
```

---

## API Reference

### Complete Method Table

| Method | Signature | Returns | Description |
|:-------|:----------|:--------|:------------|
| `new` | `FlatIndex::new(config: FlatIndexConfig)` | `FlatIndex` | Create a new index |
| `insert` | `insert(&mut self, vector: &[f32])` | `Result<u64, FlatIndexError>` | Insert a single vector |
| `insert_batch` | `insert_batch(&mut self, vectors: &[&[f32]])` | `Result<Vec<u64>, FlatIndexError>` | Insert multiple vectors |
| `search` | `search(&self, query: &[f32], k: usize)` | `Result<Vec<FlatSearchResult>, FlatIndexError>` | Exact k-NN search |
| `search_quantized` | `search_quantized(&self, query: &[f32], k: usize)` | `Result<Vec<FlatSearchResult>, FlatIndexError>` | Approximate k-NN via binary quantization |
| `get` | `get(&self, id: u64)` | `Option<&[f32]>` | Retrieve vector by ID |
| `contains` | `contains(&self, id: u64)` | `bool` | Check if vector exists |
| `delete` | `delete(&mut self, id: u64)` | `bool` | Soft-delete a vector |
| `compact` | `compact(&mut self)` | `()` | Remove deleted slots, reclaim memory |
| `enable_quantization` | `enable_quantization(&mut self)` | `Result<(), FlatIndexError>` | Build binary quantized vectors |
| `disable_quantization` | `disable_quantization(&mut self)` | `()` | Free quantized storage |
| `to_snapshot` | `to_snapshot(&self)` | `Result<Vec<u8>, PersistenceError>` | Serialize index to bytes |
| `from_snapshot` | `FlatIndex::from_snapshot(data: &[u8])` | `Result<FlatIndex, PersistenceError>` | Restore index from bytes |
| `dimensions` | `dimensions(&self)` | `u32` | Get vector dimension |
| `metric` | `metric(&self)` | `DistanceMetric` | Get distance metric |
| `len` | `len(&self)` | `usize` | Count of non-deleted vectors |
| `is_empty` | `is_empty(&self)` | `bool` | True if no non-deleted vectors |
| `capacity` | `capacity(&self)` | `usize` | Total slots including deleted |
| `config` | `config(&self)` | `&FlatIndexConfig` | Get configuration reference |
| `deleted_count` | `deleted_count(&self)` | `usize` | Count of deleted vectors |
| `deletion_ratio` | `deletion_ratio(&self)` | `f32` | Ratio of deleted to total |
| `deletion_stats` | `deletion_stats(&self)` | `(usize, usize, f32)` | Tuple: (total, deleted, ratio) |
| `is_quantized` | `is_quantized(&self)` | `bool` | True if quantization is enabled |
| `memory_usage` | `memory_usage(&self)` | `usize` | Approximate memory in bytes |

---

## FlatIndexConfig

Builder-pattern configuration for `FlatIndex`.

### new(dimensions)

```rust
FlatIndexConfig::new(dimensions: u32) -> FlatIndexConfig
```

Create a configuration with the given vector dimension.

**Defaults:**

| Field | Default Value |
|:------|:-------------|
| `metric` | `DistanceMetric::Cosine` |
| `initial_capacity` | `1000` |
| `cleanup_threshold` | `0.5` |

**Example:**

```rust
use edgevec::index::FlatIndexConfig;

let config = FlatIndexConfig::new(768); // For text-embedding-3-small
```

### with_metric(metric)

```rust
with_metric(self, metric: DistanceMetric) -> FlatIndexConfig
```

Set the distance metric.

**Available Metrics:**

| Variant | Computation | Sort Order | Use Case |
|:--------|:------------|:-----------|:---------|
| `Cosine` (default) | `dot(a,b) / (norm(a) * norm(b))` | Higher = better | Normalized embeddings |
| `DotProduct` | `sum(a[i] * b[i])` | Higher = better | Pre-normalized vectors |
| `L2` | `sqrt(sum((a[i]-b[i])^2))` | Lower = better | Spatial distance |
| `Hamming` | Count of positions where `(a!=0) != (b!=0)` | Lower = better | Binary-like vectors |

### with_capacity(capacity)

```rust
with_capacity(self, capacity: usize) -> FlatIndexConfig
```

Set the initial capacity hint for pre-allocation. The index grows automatically if this is exceeded.

### with_cleanup_threshold(threshold)

```rust
with_cleanup_threshold(self, threshold: f32) -> FlatIndexConfig
```

Set the deletion ratio that triggers automatic compaction. Value is clamped to `[0.0, 1.0]`. Default `0.5` means compaction runs when 50% of slots are deleted.

**Example:**

```rust
use edgevec::index::{FlatIndexConfig, DistanceMetric};

let config = FlatIndexConfig::new(128)
    .with_metric(DistanceMetric::L2)
    .with_capacity(5000)
    .with_cleanup_threshold(0.3);
```

---

## FlatIndex

### FlatIndex::new(config)

```rust
FlatIndex::new(config: FlatIndexConfig) -> FlatIndex
```

Create a new empty index with the given configuration.

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let config = FlatIndexConfig::new(128);
let index = FlatIndex::new(config);

assert_eq!(index.dimensions(), 128);
assert!(index.is_empty());
```

---

### insert(vector)

```rust
insert(&mut self, vector: &[f32]) -> Result<u64, FlatIndexError>
```

Insert a single vector. Returns the assigned vector ID (monotonically increasing, starting from 0).

**Errors:**
- `FlatIndexError::DimensionMismatch` if `vector.len()` does not equal the configured dimensions

**Notes:**
- Invalidates the quantized cache if quantization was enabled. Call `enable_quantization()` again after inserting.

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(3));

let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();
assert_eq!(id, 0);

let id2 = index.insert(&[4.0, 5.0, 6.0]).unwrap();
assert_eq!(id2, 1);
```

---

### insert_batch(vectors)

```rust
insert_batch(&mut self, vectors: &[&[f32]]) -> Result<Vec<u64>, FlatIndexError>
```

Insert multiple vectors. Returns the IDs assigned to each.

**Errors:**
- `FlatIndexError::DimensionMismatch` if any vector has the wrong dimension. Vectors inserted before the error remain in the index.

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(3));

let vectors: Vec<&[f32]> = vec![
    &[1.0, 2.0, 3.0],
    &[4.0, 5.0, 6.0],
];
let ids = index.insert_batch(&vectors).unwrap();

assert_eq!(ids, vec![0, 1]);
```

---

### search(query, k)

```rust
search(&self, query: &[f32], k: usize) -> Result<Vec<FlatSearchResult>, FlatIndexError>
```

Find the k nearest neighbors using exact brute-force search.

Results are sorted by relevance (best first):
- For similarity metrics (Cosine, DotProduct): highest score first
- For distance metrics (L2, Hamming): lowest score first

Returns an empty `Vec` if the index is empty.

**Errors:**
- `FlatIndexError::DimensionMismatch` if query dimension is wrong
- `FlatIndexError::InvalidK` if k is 0

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig, DistanceMetric};

let config = FlatIndexConfig::new(3).with_metric(DistanceMetric::Cosine);
let mut index = FlatIndex::new(config);

index.insert(&[1.0, 0.0, 0.0]).unwrap();
index.insert(&[0.0, 1.0, 0.0]).unwrap();

let results = index.search(&[0.9, 0.1, 0.0], 2).unwrap();
assert_eq!(results.len(), 2);
assert_eq!(results[0].id, 0); // Most similar
```

---

### get(id)

```rust
get(&self, id: u64) -> Option<&[f32]>
```

Retrieve a vector by ID. Returns `None` if the ID does not exist or was deleted.

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(3));
let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();

let v = index.get(id).unwrap();
assert_eq!(v, &[1.0, 2.0, 3.0]);

assert!(index.get(999).is_none());
```

---

### contains(id)

```rust
contains(&self, id: u64) -> bool
```

Check if a vector ID exists and is not deleted.

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(3));
let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();

assert!(index.contains(id));
assert!(!index.contains(999));
```

---

### delete(id)

```rust
delete(&mut self, id: u64) -> bool
```

Soft-delete a vector by marking it in the deletion bitmap. The vector slot is not immediately reclaimed; it is skipped during search. Call `compact()` to reclaim memory.

Returns `true` if the vector was deleted, `false` if it did not exist or was already deleted.

**Auto-compaction:** If the deletion ratio exceeds the configured `cleanup_threshold`, `compact()` is called automatically.

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(3));
let id = index.insert(&[1.0, 2.0, 3.0]).unwrap();

assert!(index.delete(id));   // Success
assert!(!index.delete(id));  // Already deleted
assert!(!index.delete(999)); // Does not exist

assert!(!index.contains(id));
```

---

### compact()

```rust
compact(&mut self)
```

Remove deleted vector slots and rebuild internal storage.

**Warning:** This operation reassigns vector IDs. After compaction, vectors are renumbered to be contiguous. Use with caution if external systems reference vector IDs.

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(
    FlatIndexConfig::new(3).with_cleanup_threshold(1.0) // Disable auto-compact
);

index.insert(&[1.0, 2.0, 3.0]).unwrap();
index.insert(&[4.0, 5.0, 6.0]).unwrap();
index.insert(&[7.0, 8.0, 9.0]).unwrap();

index.delete(1);
assert_eq!(index.len(), 2);
assert_eq!(index.capacity(), 3); // Still 3 slots

index.compact();
assert_eq!(index.capacity(), 2); // Reclaimed
```

---

## Binary Quantization

FlatIndex supports optional binary quantization (BQ) for approximate search with reduced memory usage.

### How It Works

Each f32 dimension is converted to a single bit: `value > 0.0` becomes 1, otherwise 0. This achieves 32x memory reduction for the quantized representation while the original f32 vectors are preserved.

| Storage | Per-Vector (768D) | Compression |
|:--------|:------------------|:------------|
| Original (f32) | 3,072 bytes | 1x |
| Quantized (binary) | 96 bytes | 32x |

### enable_quantization()

```rust
enable_quantization(&mut self) -> Result<(), FlatIndexError>
```

Build binary quantized vectors from the current f32 data. After enabling, use `search_quantized()` for fast approximate search.

The original f32 vectors are preserved. You can continue using `search()` for exact results.

**Notes:**
- Calling `insert()` or `delete()` after enabling quantization invalidates the quantized cache. Call `enable_quantization()` again to rebuild.
- Calling this when already enabled is a no-op.

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(8));
index.insert(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0]).unwrap();

index.enable_quantization().unwrap();
assert!(index.is_quantized());
```

### disable_quantization()

```rust
disable_quantization(&mut self)
```

Free the quantized storage. Search will use original f32 vectors only.

### search_quantized(query, k)

```rust
search_quantized(&self, query: &[f32], k: usize) -> Result<Vec<FlatSearchResult>, FlatIndexError>
```

Search using Hamming distance on binary-quantized vectors. Results are sorted by ascending Hamming distance (lower = more similar).

The query is provided as f32 values and binarized internally.

**Errors:**
- `FlatIndexError::QuantizationNotEnabled` if `enable_quantization()` was not called
- `FlatIndexError::DimensionMismatch` if query dimension is wrong
- `FlatIndexError::InvalidK` if k is 0

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(8));
index.insert(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0]).unwrap();
index.insert(&[1.0,  1.0, 1.0,  1.0, 1.0,  1.0, 1.0,  1.0]).unwrap();

index.enable_quantization().unwrap();

let results = index.search_quantized(
    &[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0], 2
).unwrap();

assert_eq!(results[0].id, 0); // Exact match: Hamming distance 0
assert!((results[0].score - 0.0).abs() < f32::EPSILON);
```

---

## Persistence

FlatIndex supports snapshot-based persistence with CRC32 integrity verification.

### Snapshot Format

| Section | Size | Description |
|:--------|:-----|:------------|
| Header length | 4 bytes (u32 LE) | Length of serialized header |
| Header | Variable (postcard) | `FlatIndexHeader` with magic, version, metadata, CRC32 |
| Bitmap length | 4 bytes (u32 LE) | Length of deletion bitmap |
| Bitmap | Variable | Deletion bitmap bytes |
| Vectors length | 8 bytes (u64 LE) | Length of vector data |
| Vectors | n * dim * 4 bytes | Little-endian f32 values |
| Quantized length | 8 bytes (u64 LE) | 0 if quantization not enabled |
| Quantized | Variable | Binary quantized vectors (optional) |

**Magic number:** `EVFI` (`[0x45, 0x56, 0x46, 0x49]`)
**Format version:** 1

### to_snapshot()

```rust
to_snapshot(&self) -> Result<Vec<u8>, PersistenceError>
```

Serialize the entire index to bytes. The output includes a CRC32 checksum computed over the data sections (bitmap, vectors, quantized).

**Errors:**
- `PersistenceError::SerializationError` if header serialization fails

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(3));
index.insert(&[1.0, 2.0, 3.0]).unwrap();

let bytes = index.to_snapshot().unwrap();
// Write `bytes` to IndexedDB, file system, etc.
```

### FlatIndex::from_snapshot(data)

```rust
FlatIndex::from_snapshot(data: &[u8]) -> Result<FlatIndex, PersistenceError>
```

Restore an index from a snapshot. Validates the magic number, format version, and CRC32 checksum before returning.

**Errors:**
- `PersistenceError::InvalidMagic` if magic number does not match
- `PersistenceError::UnsupportedVersion` if version is too new
- `PersistenceError::ChecksumMismatch` if data is corrupted
- `PersistenceError::TruncatedData` if data is incomplete
- `PersistenceError::DeserializationError` if header parsing fails

```rust
use edgevec::index::{FlatIndex, FlatIndexConfig};

let mut index = FlatIndex::new(FlatIndexConfig::new(3));
index.insert(&[1.0, 2.0, 3.0]).unwrap();

let bytes = index.to_snapshot().unwrap();
let restored = FlatIndex::from_snapshot(&bytes).unwrap();

assert_eq!(restored.len(), 1);
assert_eq!(restored.get(0).unwrap(), &[1.0, 2.0, 3.0]);
```

---

## Types

### FlatSearchResult

```rust
pub struct FlatSearchResult {
    /// Vector ID in the index.
    pub id: u64,

    /// Distance or similarity score.
    /// - For distance metrics (L2, Hamming): lower is better
    /// - For similarity metrics (Cosine, Dot): higher is better
    pub score: f32,
}
```

### DistanceMetric

```rust
#[derive(Default)]
pub enum DistanceMetric {
    #[default]
    Cosine,
    DotProduct,
    L2,
    Hamming,
}
```

The `is_similarity()` method returns `true` for `Cosine` and `DotProduct` (higher = better), and `false` for `L2` and `Hamming` (lower = better).

### FlatIndexHeader

```rust
pub struct FlatIndexHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub dimensions: u32,
    pub metric: DistanceMetric,
    pub count: u64,
    pub delete_count: u64,
    pub next_id: u64,
    pub is_quantized: bool,
    pub cleanup_threshold: f32,
    pub checksum: u32,
}
```

---

## Errors

### FlatIndexError

| Variant | Message | Cause |
|:--------|:--------|:------|
| `DimensionMismatch` | `"dimension mismatch: expected {expected}, got {actual}"` | Vector or query length does not match configured dimensions |
| `InvalidK` | `"invalid k: must be greater than 0"` | k parameter is 0 |
| `QuantizationNotEnabled` | `"quantization not enabled: call enable_quantization() first"` | `search_quantized()` called without enabling quantization |
| `EmptyIndex` | `"index is empty"` | Reserved for future use |

### PersistenceError (snapshot operations)

| Variant | Cause |
|:--------|:------|
| `InvalidMagic` | File is not a FlatIndex snapshot |
| `UnsupportedVersion` | Snapshot version is newer than this library |
| `ChecksumMismatch` | Data corruption detected |
| `TruncatedData` | File is incomplete |
| `SerializationError` | Header encoding failed |
| `DeserializationError` | Header decoding failed |

---

## Performance Characteristics

| Operation | Complexity | Notes |
|:----------|:-----------|:------|
| `insert` | O(1) | Append to contiguous storage |
| `insert_batch` | O(n) | n = number of vectors in batch |
| `search` | O(n * d) | n = vectors, d = dimensions; uses BinaryHeap for top-k |
| `search_quantized` | O(n * d/8) | 32x fewer bytes to compare via popcount |
| `get` | O(1) | Direct index into contiguous array |
| `contains` | O(1) | Bitmap lookup |
| `delete` | O(1) amortized | Bitmap set; may trigger O(n*d) compact |
| `compact` | O(n * d) | Rebuilds vector storage |
| `to_snapshot` | O(n * d) | Serializes all data + CRC32 |
| `from_snapshot` | O(n * d) | Deserializes + validates checksum |

### Memory Layout

Vectors are stored in row-major order in a single contiguous `Vec<f32>`:

```text
vectors = [v0_d0, v0_d1, ..., v0_dn, v1_d0, v1_d1, ..., v1_dn, ...]
```

Retrieval is a slice operation: `&vectors[id * dim .. (id + 1) * dim]`.

### Memory Usage

| Component | Size |
|:----------|:-----|
| Vectors (f32) | n * d * 4 bytes |
| Deletion bitmap | ceil(n / 8) bytes |
| Quantized (optional) | n * ceil(d / 8) bytes |

Example for 10,000 vectors at 768 dimensions:
- f32 storage: ~30 MB
- Bitmap: ~1.25 KB
- Quantized (if enabled): ~960 KB

---

## When to Use FlatIndex vs HNSW

| Criterion | FlatIndex | HNSW |
|:----------|:----------|:-----|
| **Dataset size** | Under 10,000 vectors | 10,000+ vectors |
| **Recall** | 100% (exact) | Approximate (tunable) |
| **Insert latency** | O(1), sub-microsecond | O(log n), graph construction |
| **Search latency** | O(n*d), scales linearly | O(log n), near-constant |
| **Memory overhead** | None (vectors + bitmap) | Graph edges (significant) |
| **Deletion** | Bitmap + compact | Complex graph repair |
| **Persistence** | Snapshot with CRC32 | Snapshot with CRC32 |

**Choose FlatIndex when:**
- You need guaranteed exact results (100% recall)
- Your dataset is small enough for linear scan (under 10k vectors)
- Insert throughput matters more than search throughput
- You are building a semantic cache with fast turnover

**Choose HNSW when:**
- Your dataset exceeds 10,000 vectors
- Approximate results are acceptable
- Search latency is the primary concern

---

## See Also

- [WasmIndex API](WASM_INDEX.md)
- [Database Operations](DATABASE_OPERATIONS.md)
- [Memory Reference](MEMORY.md)
- [Error Reference](ERROR_REFERENCE.md)
- [Filter Syntax](FILTER_SYNTAX.md)

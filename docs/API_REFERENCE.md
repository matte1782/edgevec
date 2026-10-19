# EdgeVec API Reference

**Version:** 0.3.0
**Last Updated:** 2025-12-15

This document provides a comprehensive reference for the EdgeVec API, covering both Rust and WASM/JavaScript interfaces.

---

## Core Types (Rust)

### `HnswConfig`

Configuration for HNSW index parameters.

```rust
use edgevec::HnswConfig;

let config = HnswConfig::new(128); // 128 dimensions
```

#### Fields

| Field | Type | Default | Description |
|:------|:-----|:--------|:------------|
| `dimensions` | `u32` | Required | Vector dimensionality |
| `m` | `usize` | 16 | Max connections per node |
| `m0` | `usize` | 32 | Max connections at layer 0 |
| `ef_construction` | `usize` | 200 | Build-time search width |
| `ef_search` | `usize` | 50 | Query-time search width |
| `metric` | `Metric` | `L2` | Distance metric |

### `HnswIndex`

The main HNSW index structure.

#### Constructor

```rust
pub fn new(config: HnswConfig, storage: &VectorStorage) -> Result<Self, EdgeVecError>
```

Creates a new empty index.

#### Methods

##### `insert(&mut self, vector: &[f32], storage: &mut VectorStorage) -> Result<u64, EdgeVecError>`

Insert a single vector and return its assigned ID.

**Parameters:**
- `vector`: Slice of f32 values (length must match `config.dimensions`)
- `storage`: Mutable reference to vector storage

**Returns:**
- `Ok(u64)`: The assigned vector ID
- `Err(EdgeVecError::DimensionMismatch)`: If vector length doesn't match dimensions

**Example:**
```rust
let vector = vec![0.5; 128];
let id = index.insert(&vector, &mut storage)?;
println!("Inserted vector with ID: {}", id);
```

##### `search(&self, query: &[f32], k: usize, storage: &VectorStorage) -> Result<Vec<SearchResult>, EdgeVecError>`

Search for the k nearest neighbors.

**Parameters:**
- `query`: Query vector (length must match `config.dimensions`)
- `k`: Number of neighbors to return
- `storage`: Reference to vector storage

**Returns:**
- `Ok(Vec<SearchResult>)`: Vector of results sorted by distance (ascending)
- `Err(EdgeVecError::DimensionMismatch)`: If query length doesn't match dimensions
- `Err(EdgeVecError::IndexEmpty)`: If index has no vectors

**Example:**
```rust
let query = vec![0.5; 128];
let results = index.search(&query, 10, &storage)?;

for result in results {
    println!("ID: {}, Distance: {}", result.vector_id, result.distance);
}
```

---

## Batch Insert API (Rust)

### `BatchInsertable` Trait

```rust
use edgevec::batch::BatchInsertable;
```

#### `batch_insert<P>(&mut self, vectors: Vec<(u64, Vec<f32>)>, storage: &mut VectorStorage, progress: Option<P>) -> Result<Vec<u64>, BatchError>`

Insert multiple vectors in a single batch operation.

**Type Parameters:**
- `P: Fn(usize, usize)`: Progress callback type

**Parameters:**
- `vectors`: Vector of `(id, data)` tuples
- `storage`: Mutable reference to vector storage
- `progress`: Optional progress callback `fn(inserted, total)`

**Returns:**
- `Ok(Vec<u64>)`: Vector of assigned IDs in insertion order
- `Err(BatchError)`: If batch insert fails

**Example:**
```rust
use edgevec::batch::BatchInsertable;

let vectors: Vec<(u64, Vec<f32>)> = (1..=1000)
    .map(|i| (i as u64, vec![i as f32; 128]))
    .collect();

let ids = index.batch_insert(vectors, &mut storage, Some(|inserted, total| {
    println!("Progress: {}/{}", inserted, total);
}))?;

assert_eq!(ids.len(), 1000);
```

### `BatchError`

Error type for batch operations.

| Variant | Description |
|:--------|:------------|
| `EmptyBatch` | No vectors provided |
| `DimensionMismatch { expected, got }` | Vector has wrong dimensions |
| `InsertionFailed(String)` | Individual insert failed |

---

## WASM API (JavaScript/TypeScript)

### `EdgeVecConfig`

Configuration class for WASM index.

```javascript
const config = new EdgeVecConfig(128);  // 128 dimensions
config.metric = 'cosine';               // Optional: 'l2', 'cosine', 'dot'
```

### `EdgeVec`

Main index class for WASM environment.

#### Constructor

```javascript
const index = new EdgeVec(config);
```

#### Methods

##### `insert(vector: Float32Array): number`

Insert a single vector.

```javascript
const vector = new Float32Array(128).fill(0.1);
const id = index.insert(vector);
```

##### `search(query: Float32Array, k: number): SearchResult[]`

Search for k nearest neighbors.

```javascript
const results = index.search(query, 10);
// Returns: [{ id: number, score: number }, ...]
```

##### `insertBatch(vectors: Float32Array[], config?: BatchInsertConfig): BatchInsertResult`

Batch insert multiple vectors.

```javascript
const vectors = [];
for (let i = 0; i < 100; i++) {
    vectors.push(new Float32Array(128).fill(Math.random()));
}

const result = index.insertBatch(vectors);
console.log(`Inserted ${result.inserted} vectors`);
```

##### `insertBatchWithProgress(vectors: Array<Float32Array>, onProgress: (done: number, total: number) => void): BatchInsertResult`

Batch insert with progress callback.

**Parameters:**
- `vectors`: Array of Float32Array vectors
- `onProgress`: Callback function receiving `(done, total)`

**Returns:**
- `BatchInsertResult`: Object with `inserted`, `total`, and `ids` properties

**Callback Behavior:**
- Called exactly twice: once with `(0, total)` at start, once with `(total, total)` at end
- Callback errors are intentionally ignored to ensure insert completes

**Example:**
```javascript
const vectors = Array.from({ length: 500 }, () =>
    new Float32Array(128).map(() => Math.random())
);

const result = index.insertBatchWithProgress(vectors, (done, total) => {
    const percent = Math.round(done / total * 100);
    progressBar.style.width = `${percent}%`;
    console.log(`Progress: ${percent}%`);
});

console.log(`Inserted ${result.inserted} vectors`);
console.log(`IDs: ${result.ids.slice(0, 5)}...`);
```

##### `insertBatchFlat(vectors: Float32Array, count: number): Uint32Array`

Efficient batch insert using flat array format.

```javascript
// Flat format: all vectors concatenated into single array
const dimensions = 128;
const count = 100;
const flat = new Float32Array(dimensions * count);
for (let i = 0; i < flat.length; i++) {
    flat[i] = Math.random();
}

const ids = index.insertBatchFlat(flat, count);
```

##### `save(name: string): Promise<void>`

Save the index to IndexedDB in a browser. In Node.js, use `toBytes()` and
`node:fs/promises` to write a file:

```javascript
await index.save("my-vector-db");
```

```javascript
import { writeFile, readFile } from 'node:fs/promises';

await writeFile('my-vector-db.evwb', index.toBytes());
const restored = EdgeVec.fromBytes(await readFile('my-vector-db.evwb'));
```

##### `EdgeVec.load(name: string): Promise<EdgeVec>`

Static method to load an index from IndexedDB. Both `save()` and `load()`
reject if IndexedDB is unavailable; file persistence uses `fromBytes()` above.

```javascript
const index = await EdgeVec.load("my-vector-db");
```

---

## Soft Delete API (v0.3.0)

### Rust Methods

##### `soft_delete(&mut self, vector_id: u64) -> Result<bool, EdgeVecError>`

Mark a vector as deleted (tombstoned). O(1) operation.

**Parameters:**
- `vector_id`: ID of the vector to delete

**Returns:**
- `Ok(true)`: Vector was marked deleted
- `Ok(false)`: Vector was already deleted
- `Err(EdgeVecError::InvalidInput)`: Invalid vector ID

**Example:**
```rust
let deleted = index.soft_delete(42)?;
if deleted {
    println!("Vector 42 deleted");
} else {
    println!("Vector 42 was already deleted");
}
```

##### `is_deleted(&self, vector_id: u64) -> Result<bool, EdgeVecError>`

Check if a vector has been deleted.

##### `deleted_count(&self) -> u32`

Return count of tombstoned vectors.

##### `live_count(&self) -> u32`

Return count of active (non-deleted) vectors.

##### `tombstone_ratio(&self) -> f32`

Return ratio of deleted to total vectors (0.0 to 1.0).

---

## Compaction API (v0.3.0)

### Rust Methods

##### `compact(&mut self, storage: &mut VectorStorage) -> Result<CompactionResult, EdgeVecError>`

Rebuild index removing all tombstones. Returns statistics.

**Returns:**
```rust
pub struct CompactionResult {
    pub tombstones_removed: u32,
    pub new_size: u32,
    pub duration_ms: f64,
}
```

##### `needs_compaction(&self) -> bool`

Check if tombstone ratio exceeds threshold.

##### `compaction_warning(&self) -> Option<String>`

Get warning message if compaction recommended.

##### `compaction_threshold(&self) -> f32`

Get current threshold (default: 0.3).

##### `set_compaction_threshold(&mut self, threshold: f32)`

Set threshold (0.01 to 0.99).

---

## Soft Delete API (WASM/JavaScript)

```javascript
// Soft delete a vector
const deleted = index.softDelete(42);

// Check deletion status
const isDeleted = index.isDeleted(42);

// Statistics
const deletedCount = index.deletedCount();
const liveCount = index.liveCount();
const ratio = index.tombstoneRatio();

// Compaction
const needsCompaction = index.needsCompaction();
const warning = index.compactionWarning();  // null if not needed
const result = index.compact();

// Configure threshold
const threshold = index.compactionThreshold();
index.setCompactionThreshold(0.5);  // 50%
```

### `WasmCompactionResult`

```typescript
interface WasmCompactionResult {
    tombstones_removed: number;
    new_size: number;
    duration_ms: number;
}
```

---

## Types

### `SearchResult`

Result from search operations.

**Rust:**
```rust
pub struct SearchResult {
    pub vector_id: u64,
    pub distance: f32,
}
```

**JavaScript:**
```typescript
interface SearchResult {
    id: number;
    score: number;  // Lower is closer
}
```

### `BatchInsertResult` (WASM)

Result from batch insert operations.

```typescript
interface BatchInsertResult {
    inserted: number;    // Count of successfully inserted vectors
    total: number;       // Total count attempted
    ids: number[];       // Array of assigned IDs
}
```

### `Metric`

Distance metric for similarity search.

| Value | Description | Formula |
|:------|:------------|:--------|
| `L2` | Euclidean distance | `sqrt(sum((a[i] - b[i])^2))` |
| `Cosine` | Cosine distance | `1 - (a · b) / (|a| * |b|)` |
| `Dot` | Dot product | `sum(a[i] * b[i])` |

---

## Error Handling

### `EdgeVecError` (Rust)

| Variant | Description |
|:--------|:------------|
| `DimensionMismatch { expected, got }` | Vector dimensions don't match index |
| `InvalidInput(String)` | Input validation failed |
| `IndexEmpty` | Search on empty index |
| `SerializationError(String)` | Serialization/deserialization failed |
| `IoError(String)` | File system operation failed |
| `WasmError(String)` | WASM-specific error |

### JavaScript Error Handling

WASM methods throw JavaScript exceptions on error.

```javascript
try {
    const result = index.search(query, 10);
} catch (e) {
    if (e.message.includes("DimensionMismatch")) {
        console.error("Query vector has wrong dimensions");
    }
}
```

---

## Performance Characteristics

### Batch Insert vs Sequential

| Scale | Batch Speedup | Notes |
|:------|:--------------|:------|
| 10-100 vectors | 1.2-1.5x | JS-WASM boundary overhead dominates |
| 100-1000 vectors | 1.1-1.2x | Converging as graph construction dominates |
| 1000+ vectors | ~1x | Graph construction is the bottleneck |

### Memory Usage

| Mode | Per Vector | 100k Vectors |
|:-----|:-----------|:-------------|
| Float32 | ~3,176 bytes | ~303 MB |
| Quantized (SQ8) | ~872 bytes | ~83 MB |

---

## See Also

- [README.md](../README.md) - Quick start guide
- [Competitive Analysis](benchmarks/competitive_analysis.md) - Performance comparison
- [rustdoc](https://docs.rs/edgevec) - Full Rust API documentation

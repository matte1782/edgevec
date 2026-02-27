# BinaryFlatIndex API Reference

**Version:** EdgeVec v0.9.0
**Last Updated:** 2026-02-27

A flat (brute-force) index for binary vectors. Stores vectors in a contiguous byte array for cache-friendly linear scan with SIMD-accelerated Hamming distance. Insert is O(1), search is O(n) with exact recall.

---

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
  - [Rust](#rust)
  - [JavaScript / WASM](#javascript--wasm)
- [Rust API Reference](#rust-api-reference)
  - [BinaryFlatIndex::new()](#binaryflatindexnew)
  - [BinaryFlatIndex::with_capacity()](#binaryflatindexwith_capacity)
  - [insert()](#insert)
  - [search()](#search)
  - [get()](#get)
  - [len()](#len)
  - [is_empty()](#is_empty)
  - [dimensions()](#dimensions)
  - [bytes_per_vector()](#bytes_per_vector)
  - [memory_usage()](#memory_usage)
  - [vectors_len()](#vectors_len)
  - [serialized_size()](#serialized_size)
  - [clear()](#clear)
  - [shrink_to_fit()](#shrink_to_fit)
- [HNSW Binary Methods](#hnsw-binary-methods)
  - [insert_binary()](#insert_binary)
  - [search_binary()](#search_binary)
  - [search_binary_with_ef()](#search_binary_with_ef)
- [WASM Bindings](#wasm-bindings)
  - [insertBinary()](#insertbinary)
  - [searchBinary()](#searchbinary)
  - [searchBinaryWithEf()](#searchbinarywithef)
  - [searchBinaryFiltered()](#searchbinaryfiltered)
- [Types](#types)
  - [BinaryFlatSearchResult](#binaryflatsearchresult)
  - [BinaryFlatIndexError](#binaryflatindexerror)
- [Use Cases](#use-cases)
- [Performance](#performance)
- [When to Use BinaryFlatIndex vs FlatIndex vs HNSW](#when-to-use-binaryflatindex-vs-flatindex-vs-hnsw)
- [See Also](#see-also)

---

## Overview

`BinaryFlatIndex` provides native binary vector storage using packed byte arrays. Each vector is stored as `dimensions / 8` bytes, where each bit represents one dimension. Search computes Hamming distance (number of differing bits) across all stored vectors using SIMD acceleration.

Key properties:

- **O(1) insert** -- append to contiguous buffer, ~1 us per vector
- **O(n) search** -- linear scan with SIMD Hamming distance
- **Exact recall** -- brute-force guarantees 100% recall
- **32x memory reduction** -- 1 bit per dimension vs 32 bits for f32
- **1-based IDs** -- vector IDs start at 1 (0 is a reserved sentinel)
- **Dimensions must be divisible by 8** -- required for byte-aligned packing

---

## Quick Start

### Rust

```rust
use edgevec::flat::BinaryFlatIndex;

// Create index for 1024-bit vectors (128 bytes each)
let mut index = BinaryFlatIndex::new(1024)?;

// Insert binary vectors (packed bytes)
let v1 = vec![0xFF; 128]; // All 1s
let v2 = vec![0x00; 128]; // All 0s
let id1 = index.insert(&v1)?; // Returns VectorId(1)
let id2 = index.insert(&v2)?; // Returns VectorId(2)

// Search for nearest neighbors
let query = vec![0xAA; 128];
let results = index.search(&query, 10)?;
for result in &results {
    println!("ID: {:?}, Hamming distance: {}", result.id, result.distance);
}

// Retrieve vector by ID
if let Some(bytes) = index.get(id1) {
    println!("Vector 1 has {} bytes", bytes.len());
}
```

### JavaScript / WASM

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();

// Create with Hamming metric for binary vectors
const config = new EdgeVecConfig(1024);
config.metric = 'hamming';
const db = new EdgeVec(config);

// Insert pre-packed binary vectors
const binaryVector = new Uint8Array(128); // 1024 bits = 128 bytes
const id = db.insertBinary(binaryVector);

// Search
const query = new Uint8Array(128);
const results = db.searchBinary(query, 10);
results.forEach(r => console.log(`ID: ${r.id}, Distance: ${r.score}`));
```

---

## Rust API Reference

### BinaryFlatIndex::new()

```rust
pub fn new(dimensions: usize) -> Result<Self, BinaryFlatIndexError>
```

Creates a new empty binary flat index.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `dimensions` | `usize` | Number of bits per vector. Must be divisible by 8. |

**Returns:** `Result<BinaryFlatIndex, BinaryFlatIndexError>`

**Errors:**

- `BinaryFlatIndexError::InvalidDimensions` if `dimensions` is not divisible by 8.

**Example:**

```rust
let index = BinaryFlatIndex::new(1024)?; // 1024 bits = 128 bytes per vector
assert_eq!(index.dimensions(), 1024);
assert_eq!(index.bytes_per_vector(), 128);
assert_eq!(index.len(), 0);
```

---

### BinaryFlatIndex::with_capacity()

```rust
pub fn with_capacity(dimensions: usize, capacity: usize) -> Result<Self, BinaryFlatIndexError>
```

Creates a new binary flat index with pre-allocated storage for `capacity` vectors.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `dimensions` | `usize` | Number of bits per vector. Must be divisible by 8. |
| `capacity` | `usize` | Number of vectors to pre-allocate space for. |

**Returns:** `Result<BinaryFlatIndex, BinaryFlatIndexError>`

**Errors:**

- `BinaryFlatIndexError::InvalidDimensions` if `dimensions` is not divisible by 8.
- `BinaryFlatIndexError::CapacityOverflow` if `capacity * (dimensions / 8)` overflows `usize::MAX`.

**Example:**

```rust
// Pre-allocate for 10,000 vectors of 1024 bits each
let index = BinaryFlatIndex::with_capacity(1024, 10_000)?;
assert!(index.memory_usage() >= 10_000 * 128);
```

---

### insert()

```rust
pub fn insert(&mut self, vector: &[u8]) -> Result<VectorId, BinaryFlatIndexError>
```

Inserts a binary vector into the index. The vector is appended to the contiguous storage buffer.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `vector` | `&[u8]` | Packed binary vector. Length must equal `bytes_per_vector()`. |

**Returns:** `VectorId` -- the assigned ID (1-based).

**Errors:**

- `BinaryFlatIndexError::DimensionMismatch` if `vector.len() != bytes_per_vector()`.

**Example:**

```rust
let mut index = BinaryFlatIndex::new(64)?; // 8 bytes per vector
let id = index.insert(&[0xFF; 8])?;
assert_eq!(id, VectorId(1)); // First insert gets ID 1
```

---

### search()

```rust
pub fn search(
    &self,
    query: &[u8],
    k: usize,
) -> Result<Vec<BinaryFlatSearchResult>, BinaryFlatIndexError>
```

Finds the `k` nearest neighbors using SIMD-accelerated Hamming distance. Results are sorted by distance in ascending order (lower distance = more similar).

Uses partial sort (`select_nth_unstable`) when `k < count / 10` for better performance on small k values.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `query` | `&[u8]` | Query vector as packed bytes. Length must equal `bytes_per_vector()`. |
| `k` | `usize` | Number of nearest neighbors to return. Clamped to `len()` if larger. |

**Returns:** `Vec<BinaryFlatSearchResult>` sorted by distance ascending.

Returns an empty vector if the index is empty or `k` is 0.

**Errors:**

- `BinaryFlatIndexError::DimensionMismatch` if `query.len() != bytes_per_vector()`.

**Example:**

```rust
let mut index = BinaryFlatIndex::new(64)?;
index.insert(&[0xFF; 8])?; // ID 1, all 1s
index.insert(&[0x00; 8])?; // ID 2, all 0s
index.insert(&[0x0F; 8])?; // ID 3, half 1s

let query = [0x00; 8]; // All 0s
let results = index.search(&query, 2)?;

assert_eq!(results[0].id, VectorId(2)); // Exact match, distance 0
assert_eq!(results[1].id, VectorId(3)); // 32 bits differ
```

---

### get()

```rust
pub fn get(&self, id: VectorId) -> Option<&[u8]>
```

Retrieves a vector by its ID.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `id` | `VectorId` | The 1-based vector ID returned from `insert()`. |

**Returns:** `Option<&[u8]>` -- the packed binary vector bytes, or `None` if the ID is out of bounds.

**Example:**

```rust
let mut index = BinaryFlatIndex::new(64)?;
let id = index.insert(&[0xAA; 8])?;

assert_eq!(index.get(id), Some([0xAA; 8].as_slice()));
assert_eq!(index.get(VectorId(99)), None); // Out of bounds
```

---

### len()

```rust
pub fn len(&self) -> usize
```

Returns the number of vectors in the index.

---

### is_empty()

```rust
pub fn is_empty(&self) -> bool
```

Returns `true` if the index contains no vectors.

---

### dimensions()

```rust
pub fn dimensions(&self) -> usize
```

Returns the number of bits per vector (the `dimensions` value passed to the constructor).

---

### bytes_per_vector()

```rust
pub fn bytes_per_vector(&self) -> usize
```

Returns the number of bytes per vector (`dimensions / 8`).

---

### memory_usage()

```rust
pub fn memory_usage(&self) -> usize
```

Returns the approximate total memory usage in bytes, including the struct itself and the allocated capacity of the internal buffer (not just the used portion).

---

### vectors_len()

```rust
pub fn vectors_len(&self) -> usize
```

Returns the length of the internal vectors buffer in bytes. This equals `len() * bytes_per_vector()`.

---

### serialized_size()

```rust
pub fn serialized_size(&self) -> usize
```

Estimates the serialized size in bytes. Format: 8-byte header (dimensions `u32` + count `u32`) followed by vector data.

**Example:**

```rust
let mut index = BinaryFlatIndex::new(64)?; // 8 bytes per vector
index.insert(&[0xFF; 8])?;
// Header (8) + 1 vector * 8 bytes = 16
assert_eq!(index.serialized_size(), 16);
```

---

### clear()

```rust
pub fn clear(&mut self)
```

Removes all vectors from the index. Does not release allocated memory (use `shrink_to_fit()` after clearing to release memory).

---

### shrink_to_fit()

```rust
pub fn shrink_to_fit(&mut self)
```

Shrinks the internal storage buffer to fit the current number of vectors, releasing excess allocated memory.

---

## HNSW Binary Methods

These methods are available on the HNSW index (not `BinaryFlatIndex` directly) and operate on binary vectors stored within the HNSW graph structure. They require `metric = "hamming"` in the HNSW configuration.

### insert_binary()

Inserts a pre-packed binary vector into the HNSW index. Available via the WASM `insertBinary()` binding when using HNSW mode with Hamming metric.

### search_binary()

Searches the HNSW index using a binary query vector with Hamming distance. Available via the WASM `searchBinary()` binding.

### search_binary_with_ef()

Searches the HNSW index with a custom `ef_search` parameter to tune the recall/speed tradeoff per-query. Available via the WASM `searchBinaryWithEf()` binding.

---

## WASM Bindings

All WASM binary methods require the index to be created with `metric = "hamming"`. They work with both HNSW and Flat index variants.

### insertBinary()

```typescript
insertBinary(vector: Uint8Array): number
```

Inserts a pre-packed binary vector into the index.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `vector` | `Uint8Array` | Packed binary data. Length must equal `ceil(dimensions / 8)` bytes. |

**Returns:** `number` -- the assigned vector ID (u32).

**Errors:**

- Metric is not `"hamming"`.
- Byte length does not match expected bytes for dimensions.

**Example:**

```javascript
const config = new EdgeVecConfig(1024); // 1024 bits = 128 bytes
config.metric = 'hamming';
const db = new EdgeVec(config);

const binaryVector = new Uint8Array(128); // 1024 bits packed
const id = db.insertBinary(binaryVector);
console.log('Inserted with ID:', id);
```

---

### searchBinary()

```typescript
searchBinary(query: Uint8Array, k: number): Array<{ id: number, score: number }>
```

Searches for the `k` nearest neighbors using Hamming distance.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `query` | `Uint8Array` | Binary query vector. Length must equal `ceil(dimensions / 8)`. |
| `k` | `number` | Number of nearest neighbors to return. |

**Returns:** Array of `{ id: number, score: number }` objects, where `score` is the Hamming distance (number of differing bits). Lower score = more similar.

**Errors:**

- Metric is not `"hamming"`.
- Query byte length does not match expected dimensions.

**Example:**

```javascript
const query = new Uint8Array(128);
const results = db.searchBinary(query, 10);
results.forEach(r => {
    console.log(`ID: ${r.id}, Hamming Distance: ${r.score}`);
});
```

---

### searchBinaryWithEf()

```typescript
searchBinaryWithEf(query: Uint8Array, k: number, efSearch: number): Array<{ id: number, score: number }>
```

Searches binary vectors with a custom `ef_search` parameter to tune the recall/speed tradeoff. **HNSW only** -- not available for Flat index.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `query` | `Uint8Array` | Binary query vector. |
| `k` | `number` | Number of nearest neighbors to return. |
| `efSearch` | `number` | Size of the dynamic candidate list. Must be >= `k`. |

**Returns:** Array of `{ id: number, score: number }` objects.

**Tuning guide:**

| ef_search | Recall | Speed |
|:----------|:-------|:------|
| `k` (minimum) | ~85% | Fastest |
| `2 * k` | ~90% | Fast |
| `10 * k` | ~98% | Moderate |
| `20 * k` | ~99%+ | Slowest |

**Example:**

```javascript
// Fast search, lower recall
const fast = db.searchBinaryWithEf(query, 10, 20);

// Accurate search, higher recall
const accurate = db.searchBinaryWithEf(query, 10, 200);
```

---

### searchBinaryFiltered()

```typescript
searchBinaryFiltered(query: Uint8Array, k: number, optionsJson: string): string
```

Searches binary vectors with optional metadata filtering. **HNSW only** -- not available for Flat index. Returns a JSON string that must be parsed.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `query` | `Uint8Array` | Binary query vector. |
| `k` | `number` | Maximum number of results. |
| `optionsJson` | `string` | JSON string with search options. |

**Options:**

| Field | Type | Default | Description |
|:------|:-----|:--------|:------------|
| `filter` | `string` | `null` | SQL-like filter expression. See [Filter Syntax](FILTER_SYNTAX.md). |
| `strategy` | `string` | `"auto"` | Filter strategy: `"auto"`, `"pre"`, `"post"`, `"hybrid"`. |
| `oversample_factor` | `number` | `3.0` | Oversample factor for post/hybrid strategies. |
| `include_metadata` | `boolean` | `false` | Include metadata in results. |

**Returns:** JSON string containing `{ results: [...], ... }`.

**Note:** For binary search, the strategy is internally forced to pre-filter regardless of the value passed, because post-filter and hybrid strategies could miss top-K results with fixed Hamming distance candidate sets.

**Example:**

```javascript
const query = new Uint8Array(128);
const result = JSON.parse(db.searchBinaryFiltered(query, 10, JSON.stringify({
    filter: 'category = "gpu"',
    strategy: 'auto',
    include_metadata: true
})));
console.log(`Found ${result.results.length} matches`);
result.results.forEach(r => {
    console.log(`ID: ${r.id}, Distance: ${r.score}`);
});
```

---

## Types

### BinaryFlatSearchResult

```rust
pub struct BinaryFlatSearchResult {
    /// Vector ID (1-based).
    pub id: VectorId,
    /// Hamming distance (number of differing bits). Lower = more similar.
    pub distance: f32,
}
```

In WASM, search results are returned as objects with `id` (number) and `score` (number) fields, where `score` corresponds to the Hamming distance.

### BinaryFlatIndexError

```rust
pub enum BinaryFlatIndexError {
    /// Dimensions must be divisible by 8.
    InvalidDimensions(usize),

    /// Vector length doesn't match expected bytes.
    DimensionMismatch { expected: usize, actual: usize },

    /// Capacity overflow when allocating storage.
    CapacityOverflow(usize, usize),
}
```

| Variant | Message | Cause |
|:--------|:--------|:------|
| `InvalidDimensions` | `"dimensions must be divisible by 8, got {0}"` | Dimensions not a multiple of 8. |
| `DimensionMismatch` | `"vector length {actual} doesn't match expected {expected}"` | Insert or search vector has wrong byte length. |
| `CapacityOverflow` | `"capacity overflow: {0} * {1} exceeds usize::MAX"` | `with_capacity()` allocation would overflow. |

---

## Use Cases

**Semantic caching.** Store binary hashes of embedding vectors for fast deduplication and cache lookup. Insert-heavy workloads benefit from O(1) append.

**Insert-heavy workloads.** When vectors arrive at high throughput (e.g., streaming embeddings), the ~1 us insert latency avoids backpressure that graph-based indices can create during construction.

**Exact recall required.** Brute-force scan guarantees 100% recall. Use when approximate results from HNSW are not acceptable.

**Small to medium datasets.** For datasets under ~100K vectors, the linear scan with SIMD Hamming distance is fast enough that the overhead of maintaining an HNSW graph is not justified.

**Turso f1bit_blob integration.** Binary vectors from Turso's `f1bit_blob` column type can be inserted directly without conversion.

---

## Performance

| Operation | Complexity | Typical Latency | Notes |
|:----------|:-----------|:----------------|:------|
| Insert | O(1) | ~1 us | Append to contiguous buffer. |
| Search (10K vectors) | O(n) | ~1 ms | SIMD Hamming distance. |
| Search (100K vectors) | O(n) | ~10 ms | Linear scaling. |
| Get by ID | O(1) | < 1 us | Direct offset calculation. |
| Memory per vector | -- | `dimensions / 8` bytes | 32x smaller than f32 storage. |

**Memory comparison (1024-dimensional vectors, 100K stored):**

| Storage | Per Vector | Total (100K) |
|:--------|:-----------|:-------------|
| f32 (FlatIndex) | 4,096 bytes | ~390 MB |
| Binary (BinaryFlatIndex) | 128 bytes | ~12.2 MB |

---

## When to Use BinaryFlatIndex vs FlatIndex vs HNSW

| Criteria | BinaryFlatIndex | FlatIndex | HNSW |
|:---------|:----------------|:----------|:-----|
| **Vector type** | Binary (packed bits) | f32 | f32 or binary |
| **Insert** | O(1), ~1 us | O(1) | O(log n), ~ms |
| **Search** | O(n), exact | O(n), exact | O(log n), approximate |
| **Recall** | 100% (exact) | 100% (exact) | Tunable via ef_search |
| **Memory** | 1 bit/dim | 32 bits/dim | 32 bits/dim + graph |
| **Best for** | < 100K binary vectors | < 10K f32 vectors | > 10K vectors, speed critical |
| **Metric** | Hamming only | L2, Cosine, Dot | L2, Cosine, Dot, Hamming |
| **Filtered search** | No (use HNSW) | No | Yes |
| **Persistence** | Serializable | Snapshot save/load | Snapshot save/load |

**Decision guide:**

1. If you have **binary vectors** and **< 100K entries**: use `BinaryFlatIndex`.
2. If you have **f32 vectors** and **< 10K entries**: use `FlatIndex`.
3. If you need **filtered search** or have **> 100K entries**: use HNSW with the appropriate metric.
4. If you need **approximate search with tunable recall**: use HNSW.

---

## See Also

- [WasmIndex / EdgeVec API](WASM_INDEX.md)
- [Filter Syntax Reference](FILTER_SYNTAX.md)
- [Database Operations](DATABASE_OPERATIONS.md)
- [Memory Reference](MEMORY.md)
- [Error Reference](ERROR_REFERENCE.md)

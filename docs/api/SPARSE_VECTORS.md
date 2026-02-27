# Sparse Vectors API Reference

**Version:** EdgeVec v0.9.0 (RFC-007)
**Last Updated:** 2026-02-27

---

## Overview

EdgeVec provides sparse vector storage for keyword-based retrieval features such as BM25 and TF-IDF scoring. Sparse vectors complement dense semantic embeddings in hybrid search pipelines.

Key characteristics:

- **Compressed Sparse Row (CSR) format** -- only non-zero elements are stored
- **Dot product similarity** -- higher score means more similar
- **Inverted index-ready** -- sorted indices enable efficient intersection
- **Soft deletion** -- deleted vectors are excluded from search without data compaction
- **Binary persistence** -- save/load with magic number + version validation

---

## Table of Contents

- [Data Model](#data-model)
  - [SparseVector](#sparsevector)
  - [SparseId](#sparseid)
  - [SparseSearchResult](#sparsesearchresult)
- [Rust API](#rust-api)
  - [SparseVector Constructors](#sparsevector-constructors)
  - [SparseVector Methods](#sparsevector-methods)
  - [SparseStorage](#sparsestorage)
  - [SparseSearcher](#sparsesearcher)
- [WASM / JavaScript API](#wasm--javascript-api)
  - [initSparseStorage()](#initsparseStorage)
  - [hasSparseStorage()](#hassparseStorage)
  - [sparseCount()](#sparsecount)
  - [insertSparse()](#insertsparse)
  - [searchSparse()](#searchsparse)
- [TypeScript Types](#typescript-types)
- [Use Case: BM25 Hybrid Search](#use-case-bm25-hybrid-search)
- [Performance Characteristics](#performance-characteristics)
- [Errors](#errors)
- [Limitations](#limitations)

---

## Data Model

### SparseVector

A sparse vector using Compressed Sparse Row (CSR) format. Only non-zero elements are stored.

```rust
pub struct SparseVector {
    indices: Vec<u32>,  // sorted positions of non-zero elements
    values: Vec<f32>,   // corresponding values
    dim: u32,           // maximum dimension (vocabulary size)
}
```

**Memory layout:**

| Field | Size |
|:------|:-----|
| `indices` | 8 bytes overhead + 4N bytes |
| `values` | 8 bytes overhead + 4N bytes |
| `dim` | 4 bytes |
| **Total** | ~20 bytes overhead + 8N bytes for N non-zero elements |

**Invariants enforced at construction:**

1. `indices` are sorted in strictly ascending order
2. No duplicate indices
3. No NaN or Infinity in `values`
4. At least one element (`nnz >= 1`)
5. All indices are `< dim`
6. `indices.len() == values.len()`

### SparseId

Unique identifier for sparse vectors. Uses `u64` for compatibility with dense `VectorId`. IDs are assigned monotonically and never reused.

```rust
pub struct SparseId(u64);
```

| Method | Signature | Description |
|:-------|:----------|:------------|
| `new` | `const fn new(id: u64) -> Self` | Create from u64 |
| `as_u64` | `const fn as_u64(self) -> u64` | Get underlying value |

Implements `From<u64>` and `Into<u64>`.

### SparseSearchResult

A single result from sparse search.

```rust
pub struct SparseSearchResult {
    pub id: SparseId,
    pub score: f32,     // dot product similarity
}
```

| Method | Signature | Description |
|:-------|:----------|:------------|
| `new` | `fn new(id: SparseId, score: f32) -> Self` | Create a result |

---

## Rust API

### SparseVector Constructors

#### `SparseVector::new()`

```rust
pub fn new(indices: Vec<u32>, values: Vec<f32>, dim: u32) -> Result<Self, SparseError>
```

Create a sparse vector from pre-sorted indices and values.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `indices` | `Vec<u32>` | Sorted indices of non-zero elements |
| `values` | `Vec<f32>` | Values at those indices |
| `dim` | `u32` | Maximum dimension (vocabulary size) |

**Errors:** Returns `SparseError` if any invariant is violated.

```rust
use edgevec::sparse::SparseVector;

let sparse = SparseVector::new(
    vec![1, 5, 10],
    vec![0.5, 0.3, 0.2],
    100
)?;
assert_eq!(sparse.nnz(), 3);
assert_eq!(sparse.dim(), 100);
```

---

#### `SparseVector::from_pairs()`

```rust
pub fn from_pairs(pairs: &[(u32, f32)], dim: u32) -> Result<Self, SparseError>
```

Create a sparse vector from unsorted index-value pairs. Sorts internally before validation.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `pairs` | `&[(u32, f32)]` | Index-value tuples (any order) |
| `dim` | `u32` | Maximum dimension |

```rust
use edgevec::sparse::SparseVector;

// Order does not matter -- will be sorted
let sparse = SparseVector::from_pairs(&[(10, 0.2), (0, 0.5), (5, 0.3)], 100)?;
assert_eq!(sparse.indices(), &[0, 5, 10]);
```

---

#### `SparseVector::singleton()`

```rust
pub fn singleton(index: u32, value: f32, dim: u32) -> Result<Self, SparseError>
```

Create a sparse vector with a single non-zero element. This is the minimum valid sparse vector (`nnz = 1`).

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `index` | `u32` | Index of the single element |
| `value` | `f32` | Value at that index |
| `dim` | `u32` | Maximum dimension |

```rust
use edgevec::sparse::SparseVector;

let sparse = SparseVector::singleton(42, 1.0, 100)?;
assert_eq!(sparse.nnz(), 1);
assert_eq!(sparse.indices(), &[42]);
```

---

### SparseVector Methods

| Method | Signature | Description |
|:-------|:----------|:------------|
| `indices` | `fn indices(&self) -> &[u32]` | Get sorted indices of non-zero elements |
| `values` | `fn values(&self) -> &[f32]` | Get values at those indices |
| `dim` | `fn dim(&self) -> u32` | Get maximum dimension |
| `nnz` | `fn nnz(&self) -> usize` | Number of non-zero elements |
| `get` | `fn get(&self, index: u32) -> Option<f32>` | Value at index (binary search, O(log n)) |
| `to_pairs` | `fn to_pairs(&self) -> Vec<(u32, f32)>` | Convert to index-value pairs |
| `dot` | `fn dot(&self, other: &SparseVector) -> f32` | Dot product with another sparse vector |
| `norm` | `fn norm(&self) -> f32` | L2 norm |
| `cosine` | `fn cosine(&self, other: &SparseVector) -> f32` | Cosine similarity |
| `normalize` | `fn normalize(&self) -> Result<Self, SparseError>` | Return normalized copy (unit L2 norm) |

**Dot product example:**

```rust
use edgevec::sparse::SparseVector;

let a = SparseVector::new(vec![0, 5], vec![1.0, 2.0], 100)?;
let b = SparseVector::new(vec![5, 10], vec![3.0, 1.0], 100)?;
let dot = a.dot(&b);
assert!((dot - 6.0).abs() < 1e-6); // Only index 5 overlaps: 2.0 * 3.0 = 6.0
```

**Normalization example:**

```rust
use edgevec::sparse::SparseVector;

let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100)?;
let normalized = v.normalize()?;
assert!((normalized.norm() - 1.0).abs() < 1e-6);
```

---

### SparseStorage

Packed storage for multiple sparse vectors. All vectors' indices and values are concatenated into contiguous arrays with an offset table tracking boundaries.

#### `SparseStorage::new()`

```rust
pub fn new() -> Self
```

Create empty storage.

```rust
use edgevec::sparse::SparseStorage;

let storage = SparseStorage::new();
assert!(storage.is_empty());
```

---

#### `SparseStorage::with_capacity()`

```rust
pub fn with_capacity(num_vectors: usize, avg_nnz: usize) -> Self
```

Create with pre-allocated capacity.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `num_vectors` | `usize` | Expected number of vectors |
| `avg_nnz` | `usize` | Average non-zeros per vector |

```rust
use edgevec::sparse::SparseStorage;

// Pre-allocate for 10k vectors with ~50 non-zeros each
let storage = SparseStorage::with_capacity(10_000, 50);
```

---

#### `SparseStorage::insert()`

```rust
pub fn insert(&mut self, vector: &SparseVector) -> Result<SparseId, SparseError>
```

Insert a sparse vector. Returns the assigned `SparseId`.

**Complexity:** O(nnz) time and space.

```rust
use edgevec::sparse::{SparseStorage, SparseVector};

let mut storage = SparseStorage::new();
let vector = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100)?;
let id = storage.insert(&vector)?;
assert_eq!(id.as_u64(), 0);
```

---

#### `SparseStorage::insert_batch()`

```rust
pub fn insert_batch(&mut self, vectors: &[SparseVector]) -> Result<Vec<SparseId>, SparseError>
```

Insert multiple vectors. More efficient than repeated single inserts due to reduced reallocation.

```rust
use edgevec::sparse::{SparseStorage, SparseVector};

let mut storage = SparseStorage::new();
let vectors = vec![
    SparseVector::singleton(0, 1.0, 100)?,
    SparseVector::singleton(1, 2.0, 100)?,
];
let ids = storage.insert_batch(&vectors)?;
assert_eq!(ids.len(), 2);
```

---

#### `SparseStorage::get()`

```rust
pub fn get(&self, id: SparseId) -> Option<SparseVector>
```

Retrieve a sparse vector by ID. Returns `None` if the ID is out of bounds or the vector has been deleted.

**Complexity:** O(nnz) -- reconstructs the vector from packed arrays.

```rust
use edgevec::sparse::{SparseStorage, SparseVector};

let mut storage = SparseStorage::new();
let v = SparseVector::singleton(5, 1.5, 100)?;
let id = storage.insert(&v)?;

let retrieved = storage.get(id).unwrap();
assert_eq!(retrieved.indices(), v.indices());
```

---

#### `SparseStorage::get_indices()`

```rust
pub fn get_indices(&self, id: SparseId) -> Option<&[u32]>
```

Get indices slice for a sparse vector (zero-copy). Returns `None` if ID is invalid or deleted.

**Complexity:** O(1) time and space.

---

#### `SparseStorage::get_values()`

```rust
pub fn get_values(&self, id: SparseId) -> Option<&[f32]>
```

Get values slice for a sparse vector (zero-copy). Returns `None` if ID is invalid or deleted.

**Complexity:** O(1) time and space.

---

#### `SparseStorage::get_dim()`

```rust
pub fn get_dim(&self, id: SparseId) -> Option<u32>
```

Get dimension for a sparse vector. Returns `None` if ID is invalid or deleted.

**Complexity:** O(1).

---

#### `SparseStorage::delete()`

```rust
pub fn delete(&mut self, id: SparseId) -> Result<bool, SparseError>
```

Mark a vector as deleted (soft delete). Data remains in storage but is excluded from iteration and search.

**Returns:**
- `Ok(true)` -- vector was deleted
- `Ok(false)` -- vector was already deleted
- `Err(SparseError::IdNotFound)` -- ID does not exist

```rust
use edgevec::sparse::{SparseStorage, SparseVector};

let mut storage = SparseStorage::new();
let v = SparseVector::singleton(0, 1.0, 100)?;
let id = storage.insert(&v)?;

assert!(storage.contains(id));
storage.delete(id)?;
assert!(!storage.contains(id));
```

---

#### `SparseStorage::restore()`

```rust
pub fn restore(&mut self, id: SparseId) -> Result<bool, SparseError>
```

Restore a deleted vector.

**Returns:**
- `Ok(true)` -- vector was restored
- `Ok(false)` -- vector was not deleted
- `Err(SparseError::IdNotFound)` -- ID does not exist

---

#### `SparseStorage::delete_batch()`

```rust
pub fn delete_batch(&mut self, ids: &[SparseId]) -> Result<usize, SparseError>
```

Delete multiple vectors atomically. If any ID is invalid, no deletions are performed. Returns the number of vectors actually deleted.

---

#### Query Methods

| Method | Signature | Description |
|:-------|:----------|:------------|
| `len` | `fn len(&self) -> usize` | Total vectors (including deleted) |
| `is_empty` | `fn is_empty(&self) -> bool` | True if no vectors stored |
| `live_count` | `fn live_count(&self) -> usize` | Non-deleted vectors |
| `active_count` | `fn active_count(&self) -> usize` | Alias for `live_count()` |
| `total_count` | `fn total_count(&self) -> usize` | Alias for `len()` |
| `deleted_count` | `fn deleted_count(&self) -> usize` | Number of deleted vectors |
| `deletion_ratio` | `fn deletion_ratio(&self) -> f32` | Ratio in [0.0, 1.0] |
| `memory_usage` | `fn memory_usage(&self) -> usize` | Approximate bytes used |
| `total_nnz` | `fn total_nnz(&self) -> usize` | Total non-zero elements across all vectors |
| `contains` | `fn contains(&self, id: SparseId) -> bool` | True if ID exists and is not deleted |
| `is_deleted` | `fn is_deleted(&self, id: SparseId) -> bool` | True if deleted or non-existent |
| `exists` | `fn exists(&self, id: SparseId) -> bool` | True if ID is within valid range |

---

#### Iteration

```rust
pub fn iter(&self) -> SparseStorageIter<'_>
```

Iterate over all non-deleted vectors. Yields `(SparseId, SparseVector)` pairs.

```rust
use edgevec::sparse::{SparseStorage, SparseVector};

let mut storage = SparseStorage::new();
storage.insert(&SparseVector::singleton(0, 1.0, 100)?)?;
storage.insert(&SparseVector::singleton(1, 2.0, 100)?)?;

for (id, vector) in storage.iter() {
    println!("ID {}: nnz={}", id, vector.nnz());
}
```

ID-only iteration (more efficient when vectors are not needed):

```rust
pub fn ids(&self) -> impl Iterator<Item = SparseId> + '_
```

---

#### Persistence

##### `SparseStorage::save()`

```rust
pub fn save(&self, path: &Path) -> Result<(), SparseError>
```

Save storage to a binary file with format validation header.

**Binary format:**

| Section | Size | Description |
|:--------|:-----|:------------|
| Magic | 4 bytes | `"ESPV"` in ASCII |
| Version | 4 bytes | u32 little-endian |
| Count | 8 bytes | u64 LE, number of vectors |
| Offsets | (count+1) * 4 bytes | u32 LE per offset |
| Dims | count * 4 bytes | u32 LE per vector |
| Deleted | ceil(count / 8) bytes | Packed bits |
| Next ID | 8 bytes | u64 LE |
| Total NNZ | 8 bytes | u64 LE |
| Indices | total_nnz * 4 bytes | u32 LE |
| Values | total_nnz * 4 bytes | f32 LE |

```rust
use edgevec::sparse::SparseStorage;
use std::path::Path;

let storage = SparseStorage::new();
storage.save(Path::new("sparse.espv"))?;
```

##### `SparseStorage::load()`

```rust
pub fn load(path: &Path) -> Result<Self, SparseError>
```

Load storage from a binary file. Validates magic number and format version.

**Errors:**
- `SparseError::InvalidMagic` -- file is not an ESPV file
- `SparseError::UnsupportedVersion` -- incompatible format version
- `SparseError::CorruptedData` -- data integrity check failed
- `SparseError::Io` -- file read error

```rust
use edgevec::sparse::SparseStorage;
use std::path::Path;

let storage = SparseStorage::load(Path::new("sparse.espv"))?;
println!("Loaded {} vectors", storage.len());
```

---

### SparseSearcher

Brute-force search engine over `SparseStorage` using sparse dot product similarity.

#### `SparseSearcher::new()`

```rust
pub fn new(storage: &'a SparseStorage) -> Self
```

Create a new searcher over the given storage.

---

#### `SparseSearcher::search()`

```rust
pub fn search(&self, query: &SparseVector, k: usize) -> Vec<SparseSearchResult>
```

Find the top-k most similar sparse vectors by dot product.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `query` | `&SparseVector` | Sparse query vector |
| `k` | `usize` | Number of results to return |

**Returns:** `Vec<SparseSearchResult>` sorted by descending score. May return fewer than k results if the storage has fewer live vectors or fewer vectors with non-zero overlap.

**Behavior:**
- Vectors with zero dot product (no overlapping indices) are excluded
- Deleted vectors are skipped
- Returns empty vec if k is 0 or storage is empty

**Complexity:** O(n * avg_nnz + k * log(k)) time, O(k) space.

```rust
use edgevec::sparse::{SparseStorage, SparseVector, SparseSearcher};

let mut storage = SparseStorage::new();
let v1 = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100)?;
let v2 = SparseVector::new(vec![0, 5, 20], vec![0.5, 1.5, 2.0], 100)?;
storage.insert(&v1)?;
storage.insert(&v2)?;

let searcher = SparseSearcher::new(&storage);
let query = SparseVector::new(vec![0, 5], vec![1.0, 1.0], 100)?;
let results = searcher.search(&query, 10);

for result in &results {
    println!("ID: {:?}, Score: {}", result.id, result.score);
}
```

---

#### `SparseSearcher::search_raw()`

```rust
pub fn search_raw(&self, query: &SparseVector, k: usize) -> Vec<(SparseId, f32)>
```

Search returning `(SparseId, f32)` tuples for easier integration.

---

#### `SparseSearcher::search_u64()`

```rust
pub fn search_u64(&self, query: &SparseVector, k: usize) -> Vec<(u64, f32)>
```

Search returning `(u64, f32)` tuples. Converts `SparseId` to `u64` for use with fusion algorithms that need a common ID type between dense and sparse results.

---

#### `SparseSearcher::storage()`

```rust
pub fn storage(&self) -> &SparseStorage
```

Get reference to the underlying storage.

---

## WASM / JavaScript API

The sparse API is available in JavaScript/TypeScript through WASM bindings on the `EdgeVec` class. All sparse methods require the `sparse` feature flag (enabled by default).

### initSparseStorage()

```typescript
db.initSparseStorage(): void
```

Initialize sparse storage for hybrid search. Must be called before using `insertSparse()` or `searchSparse()`. Sparse storage is lazily initialized to minimize memory footprint for users who do not need it.

Calling this method multiple times is safe -- subsequent calls are no-ops.

```javascript
const db = new EdgeVec(config);
db.initSparseStorage();
```

---

### hasSparseStorage()

```typescript
db.hasSparseStorage(): boolean
```

Check if sparse storage has been initialized.

```javascript
if (!db.hasSparseStorage()) {
    db.initSparseStorage();
}
```

---

### sparseCount()

```typescript
db.sparseCount(): number
```

Get the number of sparse vectors stored. Returns 0 if sparse storage has not been initialized.

```javascript
console.log(`Sparse vectors: ${db.sparseCount()}`);
```

---

### insertSparse()

```typescript
db.insertSparse(indices: Uint32Array, values: Float32Array, dim: number): number
```

Insert a sparse vector into storage.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `indices` | `Uint32Array` | Sparse indices, must be sorted ascending |
| `values` | `Float32Array` | Sparse values, same length as indices |
| `dim` | `number` | Dimension of the sparse space (vocabulary size) |

**Returns:** The assigned sparse vector ID as a number (f64). JavaScript numbers have integer precision up to 2^53.

**Throws:** Error if indices and values have different lengths, indices are not sorted, contain duplicates, or values are non-finite.

**Note:** If sparse storage has not been initialized, `insertSparse()` will initialize it automatically.

```javascript
db.initSparseStorage();

const indices = new Uint32Array([0, 5, 10]);
const values = new Float32Array([1.0, 2.0, 3.0]);
const id = db.insertSparse(indices, values, 10000);
console.log(`Inserted sparse vector with ID: ${id}`);
```

---

### searchSparse()

```typescript
db.searchSparse(
    indices: Uint32Array,
    values: Float32Array,
    dim: number,
    k: number
): string  // JSON
```

Search sparse vectors by dot product similarity.

**Parameters:**

| Name | Type | Description |
|:-----|:-----|:------------|
| `indices` | `Uint32Array` | Query sparse indices (sorted ascending) |
| `values` | `Float32Array` | Query sparse values |
| `dim` | `number` | Dimension of the sparse space |
| `k` | `number` | Number of results to return |

**Returns:** JSON string with array of `{ id: number, score: number }` objects, sorted by descending score.

**Throws:** Error if sparse storage has not been initialized (call `initSparseStorage()` first), or if query parameters are invalid.

```javascript
const indices = new Uint32Array([0, 5, 10]);
const values = new Float32Array([1.0, 2.0, 3.0]);
const resultsJson = db.searchSparse(indices, values, 10000, 10);
const results = JSON.parse(resultsJson);

for (const r of results) {
    console.log(`ID: ${r.id}, Score: ${r.score}`);
}
```

---

## TypeScript Types

```typescript
/** Sparse vector representation for insertSparse(). */
interface SparseVector {
    /** Sorted indices of non-zero elements (ascending). */
    indices: Uint32Array;
    /** Values at those indices (same length as indices). */
    values: Float32Array;
    /** Dimension of the sparse space (vocabulary size). */
    dim: number;
}

/** Single result from searchSparse(). */
interface SparseSearchResult {
    /** Sparse vector ID. */
    id: number;
    /** Dot product similarity score (higher = more similar). */
    score: number;
}
```

---

## Use Case: BM25 Hybrid Search

Combine dense semantic embeddings with sparse BM25 keyword features for improved retrieval quality.

### Rust

```rust
use edgevec::sparse::{SparseVector, SparseStorage, SparseSearcher};

// Vocabulary: {"machine": 0, "learning": 1, "deep": 2, "neural": 3, "network": 4}
let vocab_size = 5;

// Simulate BM25 scores for documents
let doc1_bm25 = SparseVector::from_pairs(
    &[(0, 2.3), (1, 1.8)],  // "machine learning"
    vocab_size,
)?;
let doc2_bm25 = SparseVector::from_pairs(
    &[(2, 2.1), (3, 1.5), (4, 1.9)],  // "deep neural network"
    vocab_size,
)?;
let doc3_bm25 = SparseVector::from_pairs(
    &[(0, 1.2), (2, 0.8), (1, 1.0)],  // "machine learning deep"
    vocab_size,
)?;

let mut storage = SparseStorage::new();
storage.insert(&doc1_bm25)?;
storage.insert(&doc2_bm25)?;
storage.insert(&doc3_bm25)?;

// Query: "machine learning"
let query_bm25 = SparseVector::from_pairs(
    &[(0, 2.0), (1, 1.5)],
    vocab_size,
)?;

let searcher = SparseSearcher::new(&storage);
let results = searcher.search(&query_bm25, 3);

// doc1 scores highest (exact keyword match)
// doc3 scores second (partial match)
// doc2 scores 0.0 (no overlapping terms) -- excluded
assert_eq!(results.len(), 2);
```

### JavaScript

```javascript
const db = new EdgeVec(config);
db.initSparseStorage();

// Insert BM25 scores for documents
// Vocabulary: {"machine": 0, "learning": 1, "deep": 2}
db.insertSparse(
    new Uint32Array([0, 1]),
    new Float32Array([2.3, 1.8]),
    10000
);
db.insertSparse(
    new Uint32Array([2]),
    new Float32Array([2.1]),
    10000
);

// Search for "machine learning"
const results = JSON.parse(db.searchSparse(
    new Uint32Array([0, 1]),
    new Float32Array([2.0, 1.5]),
    10000,
    10
));

// Combine with dense search results using hybridSearch()
// for best quality. See the Hybrid Search API reference.
```

---

## Performance Characteristics

### Targets (RFC-007)

| Operation | 10k vectors | 100k vectors |
|:----------|:------------|:-------------|
| **Search** | <20ms (acceptable), <10ms (target) | <100ms |
| **Insert** | P50 <50us, P99 <100us | P50 <50us, P99 <100us |
| **Get** | <1us | <1us |

All benchmarks assume average 50 non-zero elements per vector.

### Search Complexity

- **Time:** O(n * avg_nnz + k * log(k)) where n is the number of live vectors
- **Space:** O(k) for the result heap

### Memory Estimate (100k vectors, avg 50 non-zero)

| Component | Size |
|:----------|:-----|
| Packed indices | 20 MB |
| Packed values | 20 MB |
| Offsets | 0.4 MB |
| Dimensions | 0.4 MB |
| Deletion bitmap | 12.5 KB |
| **Total** | ~41 MB |

---

## Errors

All sparse operations use `SparseError`:

| Variant | Error Message | Cause |
|:--------|:-------------|:------|
| `UnsortedIndices` | `"indices must be sorted in ascending order"` | Indices not sorted |
| `DuplicateIndex(pos)` | `"duplicate index at position {pos}"` | Repeated index |
| `IndexOutOfBounds { index, dim }` | `"index {index} exceeds dimension {dim}"` | Index >= dim |
| `InvalidValue(pos)` | `"value at index {pos} is NaN or Infinity"` | Non-finite value |
| `EmptyVector` | `"sparse vector must have at least one element"` | Zero elements |
| `LengthMismatch { indices, values }` | `"indices and values length mismatch: {indices} vs {values}"` | Array lengths differ |
| `IdNotFound(id)` | `"sparse ID {id} not found"` | ID not in storage |
| `ZeroNorm` | `"cannot normalize zero vector"` | Normalizing zero-norm vector |
| `Io(msg)` | `"IO error: {msg}"` | File read/write failure |
| `InvalidMagic { expected, found }` | `"invalid magic number: ..."` | Not an ESPV file |
| `UnsupportedVersion { expected, found }` | `"unsupported format version: ..."` | Incompatible version |
| `CorruptedData(msg)` | `"corrupted data: {msg}"` | Integrity check failed |

---

## Limitations

1. **Brute-force search only.** No inverted index acceleration. Practical for collections under 100k vectors. Larger collections should consider pre-filtering or sharding.

2. **No dimension enforcement across vectors.** Each vector can have a different `dim` value. This supports multi-tenant and growing vocabulary scenarios but means the caller must ensure consistency when needed.

3. **Soft deletion only.** Deleted vectors occupy memory until a future compaction operation (not yet implemented). Monitor `deletion_ratio()` and rebuild storage if fragmentation is high.

4. **No WASM persistence for sparse storage.** Sparse vectors are typically derived from BM25/TF-IDF tokenizers and are expected to be regenerated from raw text rather than persisted. Binary `save()`/`load()` is available for native Rust only. Users can serialize `SparseStorage` separately if needed.

5. **Not thread-safe.** Wrap in `Arc<RwLock<>>` for concurrent access.

6. **No negative score results.** `searchSparse()` excludes vectors with dot product <= 0.0 from results.

7. **JavaScript ID precision.** Sparse IDs are returned as `f64` in JavaScript. Integer precision is maintained up to 2^53 (~9 quadrillion vectors).

---

## See Also

- [Database Operations](DATABASE_OPERATIONS.md) -- dense vector CRUD operations
- [Filter Syntax](FILTER_SYNTAX.md) -- metadata filtering (applies to dense search)
- [TypeScript API](TYPESCRIPT_API.md) -- complete WASM binding reference
- [Memory Management](MEMORY.md) -- memory configuration and monitoring

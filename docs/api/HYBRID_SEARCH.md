# Hybrid Search API Reference

**Version:** EdgeVec v0.9.0
**Last Updated:** 2026-02-27

---

## Overview

Hybrid search combines **dense semantic search** (HNSW index) with **sparse keyword search** (BM25/TF-IDF) and fuses the results into a single ranked list. This approach captures both semantic meaning and exact keyword matches, improving recall on queries where either method alone falls short.

```
                    Query
                   /     \
                  /       \
    +----------------+   +------------------+
    | Dense Search   |   | Sparse Search    |
    | (HNSW Index)   |   | (Inverted Index) |
    +----------------+   +------------------+
           |                      |
           | top dense_k          | top sparse_k
           |                      |
           +----------+-----------+
                      |
              +---------------+
              | Fusion        |
              | (RRF/Linear)  |
              +---------------+
                      |
                      | top final_k
                      v
              +---------------+
              | Final Results |
              +---------------+
```

Dense search finds vectors that are semantically similar to the query embedding. Sparse search finds documents that share exact terms with the query. Fusion merges both ranked lists into a single output using either Reciprocal Rank Fusion (RRF) or linear score combination.

---

## Fusion Methods

### Reciprocal Rank Fusion (RRF)

RRF combines ranked lists based on **position**, not raw score values. This makes it robust across retrieval systems with incompatible score scales.

**Formula:**

```
score(d) = sum(1 / (k + rank_i(d)))   for each list i
```

- `k` is a smoothing parameter (default: 60, per the original SIGIR 2009 paper).
- Higher `k` values give more weight to documents ranked lower in lists.
- Ranks are 1-indexed.

**When to use:** RRF is the recommended default. It requires no tuning, handles score scale differences automatically, and produces stable results across different query types.

**Reference:** Cormack, G.V., Clarke, C.L.A., and Buettcher, S. (2009). "Reciprocal Rank Fusion outperforms Condorcet and individual Rank Learning Methods." SIGIR 2009.

### Linear Combination

Linear fusion normalizes scores from each list to `[0, 1]` using min-max normalization, then combines them with a weighted sum.

**Formula:**

```
score(d) = alpha * norm_dense(d) + (1 - alpha) * norm_sparse(d)
```

- `alpha` controls the weight given to dense scores (`0.0` = sparse only, `1.0` = dense only, `0.5` = equal weight).
- Documents appearing in only one list receive `0.0` for the missing component.
- Normalization uses `(score - min) / (max - min)`. If all scores are identical, they normalize to `1.0`.

**When to use:** Linear combination is useful when you want explicit control over the dense/sparse tradeoff, or when you have tuned `alpha` on a validation set for your domain.

**Important:** Linear fusion expects non-negative scores. Dense cosine similarity in `[0, 1]` and sparse BM25/TF-IDF scores (`>= 0`) satisfy this requirement. Behavior is undefined for negative scores.

---

## Quick Start

### Rust

```rust
use edgevec::hybrid::{HybridSearcher, HybridSearchConfig, FusionMethod};
use edgevec::sparse::SparseVector;

// Assume hnsw_index, dense_storage, and sparse_storage are already built.
let searcher = HybridSearcher::new(&hnsw_index, &dense_storage, &sparse_storage);

// Dense query: a 128-dimensional embedding from your model
let dense_query: Vec<f32> = vec![0.1, 0.2, /* ... 128 values total */];

// Sparse query: BM25 term weights (indices = token IDs, values = BM25 scores)
let sparse_query = SparseVector::new(
    vec![42, 187, 1024],        // token indices
    vec![2.3, 1.8, 0.9],       // BM25 scores
    10000,                       // vocabulary size
).expect("valid sparse vector");

// Search with RRF fusion: retrieve 20 from each, return top 10
let config = HybridSearchConfig::rrf(20, 20, 10);
let results = searcher.search(&dense_query, &sparse_query, &config)
    .expect("search succeeds");

for result in &results {
    println!(
        "ID: {:?}, Score: {:.4}, Dense Rank: {:?}, Sparse Rank: {:?}",
        result.id, result.score, result.dense_rank, result.sparse_rank
    );
}
```

### JavaScript (WASM)

```javascript
import { EdgeVec } from 'edgevec';

// Assume db is an initialized EdgeVec instance with HNSW index
// and sparse storage has been initialized via db.initSparseStorage()

const denseQuery = new Float32Array([0.1, 0.2, /* ... 128 values */]);
const sparseIndices = new Uint32Array([42, 187, 1024]);
const sparseValues = new Float32Array([2.3, 1.8, 0.9]);
const sparseDim = 10000; // vocabulary size

const results = JSON.parse(db.hybridSearch(
    denseQuery,
    sparseIndices,
    sparseValues,
    sparseDim,
    JSON.stringify({ k: 10, fusion: 'rrf' })
));

for (const r of results) {
    console.log(`ID: ${r.id}, Score: ${r.score.toFixed(4)}`);
}
```

---

## Complete API Reference (Rust)

### FusionMethod

Enum controlling how dense and sparse results are combined.

```rust
pub enum FusionMethod {
    Rrf { k: u32 },
    Linear { alpha: f32 },
}
```

#### Variants

| Variant | Fields | Description |
|:--------|:-------|:------------|
| `Rrf` | `k: u32` | Reciprocal Rank Fusion with smoothing parameter `k` |
| `Linear` | `alpha: f32` | Weighted linear combination; `alpha` in `[0.0, 1.0]` |

#### Constructors

**`FusionMethod::rrf() -> Self`**

Creates RRF fusion with the default `k=60`.

```rust
let fusion = FusionMethod::rrf();
// FusionMethod::Rrf { k: 60 }
```

**`FusionMethod::rrf_with_k(k: u32) -> Self`**

Creates RRF fusion with a custom `k` parameter.

```rust
let fusion = FusionMethod::rrf_with_k(100);
// FusionMethod::Rrf { k: 100 }
```

**`FusionMethod::linear(alpha: f32) -> Self`**

Creates linear combination fusion with the given `alpha` weight.

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `alpha` | `f32` | Weight for dense scores. `0.0` = sparse only, `1.0` = dense only. |

**Panics** if `alpha` is not in the range `[0.0, 1.0]`.

```rust
let fusion = FusionMethod::linear(0.7); // 70% dense, 30% sparse
```

**`FusionMethod::linear_balanced() -> Self`**

Creates linear combination fusion with `alpha=0.5` (equal weight).

```rust
let fusion = FusionMethod::linear_balanced();
// FusionMethod::Linear { alpha: 0.5 }
```

**`FusionMethod::default() -> Self`**

Returns `FusionMethod::Rrf { k: 60 }`.

---

### HybridSearchConfig

Configuration for hybrid search. Controls how many results to retrieve from each search type and how to fuse them.

```rust
pub struct HybridSearchConfig {
    pub dense_k: usize,
    pub sparse_k: usize,
    pub final_k: usize,
    pub fusion: FusionMethod,
}
```

#### Fields

| Field | Type | Description |
|:------|:-----|:------------|
| `dense_k` | `usize` | Number of results to retrieve from dense (HNSW) search. More candidates improve recall but increase latency. |
| `sparse_k` | `usize` | Number of results to retrieve from sparse search. More candidates improve recall but increase latency. |
| `final_k` | `usize` | Final number of results to return after fusion. |
| `fusion` | `FusionMethod` | Fusion method to combine results. |

#### Constructors

**`HybridSearchConfig::new(dense_k: usize, sparse_k: usize, final_k: usize, fusion: FusionMethod) -> Self`**

Creates a new configuration with explicit parameters.

```rust
let config = HybridSearchConfig::new(20, 20, 10, FusionMethod::rrf());
```

**`HybridSearchConfig::rrf(dense_k: usize, sparse_k: usize, final_k: usize) -> Self`**

Creates a configuration with RRF fusion (`k=60`).

```rust
let config = HybridSearchConfig::rrf(20, 20, 10);
```

**`HybridSearchConfig::rrf_with_k(dense_k: usize, sparse_k: usize, final_k: usize, rrf_k: u32) -> Self`**

Creates a configuration with RRF fusion using a custom `k` parameter.

```rust
let config = HybridSearchConfig::rrf_with_k(20, 20, 10, 100);
```

**`HybridSearchConfig::linear(dense_k: usize, sparse_k: usize, final_k: usize, alpha: f32) -> Self`**

Creates a configuration with linear fusion.

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `alpha` | `f32` | Weight for dense scores (`0.0` = sparse only, `1.0` = dense only) |

```rust
let config = HybridSearchConfig::linear(50, 50, 10, 0.7);
```

**`HybridSearchConfig::default() -> Self`**

Returns a default configuration: `dense_k=20`, `sparse_k=20`, `final_k=10`, RRF fusion.

#### Methods

**`validate(&self) -> Result<(), String>`**

Validates the configuration.

Returns `Err` if:
- Both `dense_k` and `sparse_k` are `0`
- `final_k` is `0`

```rust
let config = HybridSearchConfig::default();
assert!(config.validate().is_ok());
```

---

### HybridSearcher

Orchestrates dense and sparse search, then fuses results.

```rust
pub struct HybridSearcher<'a> {
    // borrows HnswIndex, VectorStorage, SparseStorage
}
```

#### Constructor

**`HybridSearcher::new(index: &'a HnswIndex, dense_storage: &'a VectorStorage, sparse_storage: &'a SparseStorage) -> Self`**

Creates a new hybrid searcher.

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `index` | `&HnswIndex` | HNSW index for dense search |
| `dense_storage` | `&VectorStorage` | Vector storage backing the HNSW index |
| `sparse_storage` | `&SparseStorage` | Sparse vector storage for keyword search |

**Note:** In debug builds, a warning is printed to stderr if `dense_storage.len() != sparse_storage.len()`, as this may indicate ID misalignment between dense and sparse vectors.

```rust
let searcher = HybridSearcher::new(&hnsw_index, &dense_storage, &sparse_storage);
```

#### Methods

**`search(&self, dense_query: &[f32], sparse_query: &SparseVector, config: &HybridSearchConfig) -> Result<Vec<HybridSearchResult>, HybridError>`**

Performs hybrid search combining dense and sparse retrieval.

**Algorithm:**

1. Execute dense search via HNSW index (retrieves `config.dense_k` results)
2. Execute sparse search via brute-force (retrieves `config.sparse_k` results)
3. Convert results to common ID format (`u64`)
4. Fuse results using the configured method (RRF or Linear)
5. Return top `config.final_k` fused results

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `dense_query` | `&[f32]` | Dense embedding vector for HNSW search |
| `sparse_query` | `&SparseVector` | Sparse vector for keyword search |
| `config` | `&HybridSearchConfig` | Search configuration |

**Returns:** `Vec<HybridSearchResult>` sorted by fused score (descending).

**Errors:**

| Error | Cause |
|:------|:------|
| `HybridError::DimensionMismatch` | Dense query dimension does not match index |
| `HybridError::InvalidConfig` | Config validation fails (e.g., both `dense_k` and `sparse_k` are 0) |
| `HybridError::DenseSearchError` | Internal HNSW search failure |

---

**`search_dense_only(&self, dense_query: &[f32], k: usize) -> Result<Vec<HybridSearchResult>, HybridError>`**

Searches using only the dense (HNSW) index. Sparse search is disabled. Useful for A/B testing or when sparse features are unavailable.

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `dense_query` | `&[f32]` | Dense embedding vector |
| `k` | `usize` | Number of results to return |

---

**`search_sparse_only(&self, sparse_query: &SparseVector, k: usize) -> Result<Vec<HybridSearchResult>, HybridError>`**

Searches using only the sparse index. Dense search is disabled. Useful for keyword-only search or A/B testing.

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `sparse_query` | `&SparseVector` | Sparse vector for keyword search |
| `k` | `usize` | Number of results to return |

---

**`components(&self) -> (&HnswIndex, &VectorStorage, &SparseStorage)`**

Returns references to the underlying index, dense storage, and sparse storage.

---

**`dense_count(&self) -> usize`**

Returns the number of vectors in dense storage.

---

**`sparse_count(&self) -> usize`**

Returns the number of vectors in sparse storage.

---

### HybridSearchResult

Result from hybrid search, containing the fused ranking with original rank and score information from both retrieval systems.

```rust
pub struct HybridSearchResult {
    pub id: VectorId,
    pub score: f32,
    pub dense_rank: Option<usize>,
    pub dense_score: Option<f32>,
    pub sparse_rank: Option<usize>,
    pub sparse_score: Option<f32>,
}
```

| Field | Type | Description |
|:------|:-----|:------------|
| `id` | `VectorId` | Vector ID |
| `score` | `f32` | Combined score from fusion |
| `dense_rank` | `Option<usize>` | Original rank in dense results (1-indexed). `None` if not found in dense results. |
| `dense_score` | `Option<f32>` | Original dense similarity score. `None` if not found in dense results. |
| `sparse_rank` | `Option<usize>` | Original rank in sparse results (1-indexed). `None` if not found in sparse results. |
| `sparse_score` | `Option<f32>` | Original sparse score. `None` if not found in sparse results. |

**Note on Equality:** This type derives `PartialEq`, which compares `f32` scores directly. Due to floating-point precision, two results with nearly-identical scores may not compare equal. For score comparison, use an epsilon-based comparison.

---

### HybridError

Errors that can occur during hybrid search.

```rust
pub enum HybridError {
    InvalidConfig(String),
    DenseSearchError(String),
    DimensionMismatch { expected: usize, actual: usize },
}
```

| Variant | Description |
|:--------|:------------|
| `InvalidConfig(String)` | Configuration validation failed |
| `DenseSearchError(String)` | Dense search failed |
| `DimensionMismatch { expected, actual }` | Query dimensions do not match the index |

`HybridError` implements `std::error::Error` and `Display`. It also implements `From<GraphError>`, converting HNSW graph errors automatically.

---

### Standalone Fusion Functions

These functions are re-exported from `edgevec::hybrid` and can be used independently of `HybridSearcher` if you have pre-computed result lists.

**`rrf_fusion(dense_results: &[(u64, f32)], sparse_results: &[(u64, f32)], k: u32, top_n: usize) -> Vec<FusionResult>`**

Reciprocal Rank Fusion on two ranked lists.

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `dense_results` | `&[(u64, f32)]` | Dense results as `(id, score)` tuples, ordered by descending score. Scores are unused; only rank matters. |
| `sparse_results` | `&[(u64, f32)]` | Sparse results as `(id, score)` tuples, ordered by descending score. Scores are unused; only rank matters. |
| `k` | `u32` | RRF smoothing parameter (standard: 60) |
| `top_n` | `usize` | Number of results to return |

**Returns:** `Vec<FusionResult>` sorted by descending RRF score.

**Complexity:** O(d + s + u log u) time, O(u) space, where d = |dense|, s = |sparse|, u = unique IDs.

```rust
use edgevec::hybrid::rrf_fusion;

let dense = vec![(1, 0.95), (2, 0.80), (3, 0.75)];
let sparse = vec![(2, 5.5), (4, 4.2), (1, 3.8)];

let results = rrf_fusion(&dense, &sparse, 60, 10);
// ID 2 appears in both lists at good ranks -> highest fused score
assert_eq!(results[0].id, 2);
```

---

**`linear_fusion(dense_results: &[(u64, f32)], sparse_results: &[(u64, f32)], alpha: f32, top_n: usize) -> Vec<FusionResult>`**

Linear combination fusion with min-max normalization.

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `dense_results` | `&[(u64, f32)]` | Dense results as `(id, score)` tuples |
| `sparse_results` | `&[(u64, f32)]` | Sparse results as `(id, score)` tuples |
| `alpha` | `f32` | Weight for dense scores (`0.0` to `1.0`). Clamped if out of range. |
| `top_n` | `usize` | Number of results to return |

**Returns:** `Vec<FusionResult>` sorted by descending combined score.

```rust
use edgevec::hybrid::linear_fusion;

let dense = vec![(1, 0.95), (2, 0.80)];
let sparse = vec![(2, 5.0), (1, 3.0)];

let results = linear_fusion(&dense, &sparse, 0.5, 10);
```

---

### FusionResult

Result from standalone fusion functions.

```rust
pub struct FusionResult {
    pub id: u64,
    pub score: f32,
    pub dense_rank: Option<usize>,
    pub sparse_rank: Option<usize>,
}
```

| Field | Type | Description |
|:------|:-----|:------------|
| `id` | `u64` | Document/vector ID |
| `score` | `f32` | Combined score from fusion |
| `dense_rank` | `Option<usize>` | Original rank in dense results (1-indexed, `None` if not present) |
| `sparse_rank` | `Option<usize>` | Original rank in sparse results (1-indexed, `None` if not present) |

---

### Constants

**`RRF_DEFAULT_K: u32 = 60`**

Default `k` parameter for RRF fusion, from the original SIGIR 2009 paper.

---

## WASM Binding

### `hybridSearch()`

```typescript
hybridSearch(
    dense_query: Float32Array,
    sparse_indices: Uint32Array,
    sparse_values: Float32Array,
    sparse_dim: number,
    options_json: string
): string
```

Performs hybrid search combining dense and sparse retrieval. Requires the `sparse` feature.

**Prerequisites:** Sparse storage must be initialized via `initSparseStorage()` before calling this method.

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `dense_query` | `Float32Array` | Dense embedding vector. Length must match the index dimensions. |
| `sparse_indices` | `Uint32Array` | Sparse query indices (token IDs). Must be sorted. |
| `sparse_values` | `Float32Array` | Sparse query values (BM25/TF-IDF scores). Must have the same length as `sparse_indices`. |
| `sparse_dim` | `number` | Dimension of sparse space (vocabulary size). |
| `options_json` | `string` | JSON configuration string (see below). |

**Returns:** JSON string containing an array of result objects.

**Throws** on:
- Sparse storage not initialized
- Dense query dimension mismatch
- `sparse_indices` and `sparse_values` length mismatch
- Invalid options JSON
- Dense query contains non-finite values

#### Options JSON Schema

```json
{
    "k": 10,
    "dense_k": 20,
    "sparse_k": 20,
    "fusion": "rrf"
}
```

| Field | Type | Required | Default | Description |
|:------|:-----|:---------|:--------|:------------|
| `k` | `number` | Yes | -- | Final number of results to return |
| `dense_k` | `number` | No | `20` | Number of dense results to retrieve |
| `sparse_k` | `number` | No | `20` | Number of sparse results to retrieve |
| `fusion` | `string` or `object` | No | `"rrf"` | Fusion method (see below) |

**Fusion field values:**

| Value | Description |
|:------|:------------|
| `"rrf"` | Reciprocal Rank Fusion with `k=60` |
| `{ "type": "linear", "alpha": 0.7 }` | Linear combination with specified alpha |

#### Result JSON Schema

```json
[
    {
        "id": 42,
        "score": 0.032,
        "dense_rank": 1,
        "dense_score": 0.95,
        "sparse_rank": 3,
        "sparse_score": 4.2
    }
]
```

| Field | Type | Always Present | Description |
|:------|:-----|:---------------|:------------|
| `id` | `number` | Yes | Vector ID |
| `score` | `number` | Yes | Combined fusion score |
| `dense_rank` | `number` | No | Rank in dense results (1-indexed). Absent if not found in dense results. |
| `dense_score` | `number` | No | Original dense similarity score. Absent if not found in dense results. |
| `sparse_rank` | `number` | No | Rank in sparse results (1-indexed). Absent if not found in sparse results. |
| `sparse_score` | `number` | No | Original sparse score. Absent if not found in sparse results. |

---

## TypeScript Types

```typescript
/** Options for hybridSearch() */
interface HybridSearchOptions {
    /** Final number of results to return (required) */
    k: number;
    /** Number of dense results to retrieve (default: 20) */
    dense_k?: number;
    /** Number of sparse results to retrieve (default: 20) */
    sparse_k?: number;
    /** Fusion method: "rrf" or { type: "linear", alpha: number } */
    fusion?: FusionMethod;
}

/** Fusion method configuration */
type FusionMethod =
    | 'rrf'
    | { type: 'linear'; alpha: number };

/** Single result from hybridSearch() */
interface HybridSearchResult {
    /** Vector ID */
    id: number;
    /** Combined fusion score (higher = more relevant) */
    score: number;
    /** Rank in dense results (1-indexed), absent if not in dense results */
    dense_rank?: number;
    /** Original dense similarity score, absent if not in dense results */
    dense_score?: number;
    /** Rank in sparse results (1-indexed), absent if not in sparse results */
    sparse_rank?: number;
    /** Original sparse score, absent if not in sparse results */
    sparse_score?: number;
}
```

---

## End-to-End Example: Dense Embeddings + BM25

This example shows a complete pipeline combining a dense embedding model with BM25 sparse scores for document search.

### Rust

```rust
use edgevec::hybrid::{HybridSearcher, HybridSearchConfig};
use edgevec::sparse::SparseVector;

// --- Setup Phase ---
// Assume you have already:
// 1. Built an HnswIndex and VectorStorage with dense embeddings
// 2. Built a SparseStorage with BM25 term-weight vectors
// 3. Inserted documents in the same order to both stores (ID alignment)

let searcher = HybridSearcher::new(&hnsw_index, &dense_storage, &sparse_storage);

// --- Query Phase ---
// Dense: get embedding from your model (e.g., text-embedding-3-small, 1536-dim)
let dense_query: Vec<f32> = embedding_model.encode("rust async runtime");

// Sparse: compute BM25 scores for query terms
// Token "rust" -> index 4521, BM25 = 3.2
// Token "async" -> index 872, BM25 = 2.8
// Token "runtime" -> index 6103, BM25 = 1.9
let sparse_query = SparseVector::new(
    vec![872, 4521, 6103],
    vec![2.8, 3.2, 1.9],
    30000, // vocabulary size
).expect("valid sparse vector");

// --- Search ---
// RRF fusion: robust default, no tuning needed
let config = HybridSearchConfig::rrf(50, 50, 10);
let results = searcher.search(&dense_query, &sparse_query, &config)?;

for r in &results {
    println!("ID: {:?}", r.id);
    println!("  Fused score: {:.4}", r.score);
    if let (Some(dr), Some(ds)) = (r.dense_rank, r.dense_score) {
        println!("  Dense:  rank={}, score={:.4}", dr, ds);
    }
    if let (Some(sr), Some(ss)) = (r.sparse_rank, r.sparse_score) {
        println!("  Sparse: rank={}, score={:.4}", sr, ss);
    }
    println!();
}
```

### JavaScript

```javascript
// --- Setup Phase ---
// Assume db is an EdgeVec instance with HNSW index and sparse storage initialized

// --- Query Phase ---
// Dense: embedding from your model
const denseQuery = new Float32Array(await embeddingModel.encode("rust async runtime"));

// Sparse: BM25 term weights from your tokenizer
// Token "async" -> index 872, BM25 = 2.8
// Token "rust" -> index 4521, BM25 = 3.2
// Token "runtime" -> index 6103, BM25 = 1.9
const sparseIndices = new Uint32Array([872, 4521, 6103]);
const sparseValues = new Float32Array([2.8, 3.2, 1.9]);
const vocabSize = 30000;

// --- Search with RRF ---
const results = JSON.parse(db.hybridSearch(
    denseQuery,
    sparseIndices,
    sparseValues,
    vocabSize,
    JSON.stringify({
        k: 10,
        dense_k: 50,
        sparse_k: 50,
        fusion: 'rrf'
    })
));

for (const r of results) {
    console.log(`ID: ${r.id}, Fused: ${r.score.toFixed(4)}`);
    if (r.dense_rank !== undefined) {
        console.log(`  Dense:  rank=${r.dense_rank}, score=${r.dense_score.toFixed(4)}`);
    }
    if (r.sparse_rank !== undefined) {
        console.log(`  Sparse: rank=${r.sparse_rank}, score=${r.sparse_score.toFixed(4)}`);
    }
}

// --- Search with Linear Fusion ---
const linearResults = JSON.parse(db.hybridSearch(
    denseQuery,
    sparseIndices,
    sparseValues,
    vocabSize,
    JSON.stringify({
        k: 10,
        dense_k: 50,
        sparse_k: 50,
        fusion: { type: 'linear', alpha: 0.7 }
    })
));
```

---

## ID Alignment Contract

Hybrid search requires that the same document has **matching numeric IDs** in both dense and sparse storage. If IDs do not align, fusion treats them as different documents and search quality degrades significantly.

Three strategies for maintaining alignment:

| Strategy | Description | Best For |
|:---------|:------------|:---------|
| **Insert Order** | Insert dense and sparse vectors in the same order so auto-assigned IDs match | Simple use cases |
| **Explicit IDs** | Use `insert_with_id()` methods to set matching IDs explicitly | Complex pipelines |
| **Mapping Layer** | Maintain an external `document_id -> (VectorId, SparseId)` mapping | Legacy systems |

---

## Performance Characteristics

| Operation | Complexity | Notes |
|:----------|:-----------|:------|
| Dense search (HNSW) | O(log n) | Approximate nearest neighbor |
| Sparse search | O(n) | Brute-force scan |
| RRF fusion | O(d + s + u log u) | d = dense_k, s = sparse_k, u = unique IDs |
| Linear fusion | O(d + s + u log u) | Includes normalization pass |
| Score lookup | O(1) | HashMap-based |

Fusion overhead is negligible compared to the search phases. For typical configurations (`dense_k=20`, `sparse_k=20`), fusion adds microseconds.

**Tuning `dense_k` and `sparse_k`:** Setting these higher than `final_k` improves recall at the cost of latency. A 2x to 5x ratio of `dense_k/final_k` is a reasonable starting point. RRF is less sensitive to this ratio than linear fusion.

---

## When to Use Each Search Mode

| Mode | Best For | Tradeoffs |
|:-----|:---------|:----------|
| **Hybrid (RRF)** | General-purpose search; queries mixing semantic intent and keywords | Best recall; no tuning needed; slight overhead from two searches |
| **Hybrid (Linear)** | Domain-tuned search where you have validation data to set alpha | Requires tuning; can outperform RRF when alpha is well-calibrated |
| **Dense only** | Semantic similarity; queries where meaning matters more than exact terms | Misses exact keyword matches; faster than hybrid |
| **Sparse only** | Exact keyword matching; structured or domain-specific terminology | No semantic understanding; fast for keyword-heavy queries |

**Recommendation:** Start with RRF hybrid search (`HybridSearchConfig::rrf(20, 20, 10)`). Switch to dense-only or sparse-only if benchmarking shows hybrid does not improve your specific workload.

---

## See Also

- [Sparse Search API](SPARSE_SEARCH.md)
- [HNSW Index API](WASM_INDEX.md)
- [Filter Syntax](FILTER_SYNTAX.md)
- [TypeScript API](TYPESCRIPT_API.md)

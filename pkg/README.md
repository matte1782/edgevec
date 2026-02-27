# EdgeVec

[![CI](https://github.com/matte1782/edgevec/actions/workflows/ci.yml/badge.svg)](https://github.com/matte1782/edgevec/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/edgevec.svg)](https://crates.io/crates/edgevec)
[![npm](https://img.shields.io/npm/v/edgevec.svg)](https://www.npmjs.com/package/edgevec)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/matte1782/edgevec/blob/main/LICENSE-MIT)

> **The first WASM-native vector database.**
> Dense + sparse vectors, hybrid search, binary quantization, metadata filtering — all in the browser.

EdgeVec is an embedded vector database built in Rust with first-class WebAssembly support. It brings server-grade vector database features to the browser: **HNSW + FlatIndex**, **sparse vectors with BM25**, **hybrid search (RRF fusion)**, **32x memory reduction** via binary quantization, metadata filtering, soft delete, persistence, and sub-millisecond search.

---

## Why EdgeVec?

| Feature | EdgeVec | hnswlib-wasm | Pinecone |
|:--------|:-------:|:------------:|:--------:|
| Vector Search | Yes | Yes | Yes |
| **Binary Quantization** | **Yes (32x)** | No | No |
| **Metadata Filtering** | **Yes** | No | Yes |
| **Sparse Vectors** | **Yes** | No | Yes |
| **Hybrid Search (RRF)** | **Yes** | No | Yes |
| **FlatIndex (exact)** | **Yes** | No | Limited |
| **BinaryFlatIndex** | **Yes** | No | No |
| **SQL-like Queries** | **Yes** | No | Yes |
| **Memory Pressure API** | **Yes** | No | No |
| **Soft Delete** | **Yes** | No | Yes |
| **Persistence** | **Yes** | No | Yes |
| Browser-native | Yes | Yes | No |
| No server required | Yes | Yes | No |
| Offline capable | Yes | Yes | No |

**EdgeVec is the only WASM vector database with binary quantization, sparse vectors, hybrid search, BinaryFlatIndex, and filtered search.**

---

## Try It Now

Build filters visually, see live results, copy-paste ready code:

**[Filter Playground](https://matte1782.github.io/edgevec/demo/)** - Interactive filter builder with live sandbox

- Visual filter construction
- 10 ready-to-use examples
- Live WASM execution
- Copy-paste code snippets (JS/TS/React)

---

## Quick Start

```bash
npm install edgevec
```

```typescript
import init, { EdgeVec } from 'edgevec';

await init();

// Create index (768D for embeddings like OpenAI, Cohere)
const db = new EdgeVec({ dimensions: 768 });

// Insert vectors with metadata (v0.6.0)
const vector = new Float32Array(768).map(() => Math.random());
const id = db.insertWithMetadata(vector, {
    category: "books",
    price: 29.99,
    inStock: true
});

// Search with filter expression (v0.6.0)
const query = new Float32Array(768).map(() => Math.random());
const results = db.searchWithFilter(query, 'category = "books" AND price < 50', 10);

// Fast BQ search with rescoring — 32x less memory, 95% recall (v0.6.0)
const fastResults = db.searchBQ(query, 10);

// Monitor memory pressure (v0.6.0)
const pressure = db.getMemoryPressure();
if (pressure.level === 'warning') {
    db.compact();  // Free deleted vectors
}
```

---

## Interactive Demos

Try EdgeVec directly in your browser:

| Demo | Description |
|:-----|:------------|
| [**Filter Playground v0.7.0**](https://matte1782.github.io/edgevec/demo/) | Visual filter builder with live sandbox (NEW!) |
| [**v0.6.0 Cyberpunk Demo**](https://matte1782.github.io/edgevec/demo/cyberpunk.html) | BQ vs F32 comparison, metadata filtering, memory pressure |
| [**Demo Hub**](https://matte1782.github.io/edgevec/demo/hub.html) | All demos in one place |

**Run locally:**
| Demo | Path |
|:-----|:-----|
| SIMD Benchmark | `wasm/examples/simd_benchmark.html` |
| Benchmark Dashboard | `wasm/examples/benchmark-dashboard.html` |
| Soft Delete Demo | `wasm/examples/soft_delete.html` |
| Main Demo | `wasm/examples/index.html` |

```bash
# Run demos locally
git clone https://github.com/matte1782/edgevec.git
cd edgevec
python -m http.server 8080
# Open http://localhost:8080/wasm/examples/index.html
```

---

## Performance

EdgeVec v0.9.0 uses **SIMD instructions** for 2x+ faster vector operations on modern browsers.

### Distance Calculation (Native Benchmark)

| Dimension | Dot Product | L2 Distance | Throughput |
|:----------|:------------|:------------|:-----------|
| 128 | 55 ns | 66 ns | 2.3 Gelem/s |
| 384 | 188 ns | 184 ns | 2.1 Gelem/s |
| 768 | 374 ns | 358 ns | 2.1 Gelem/s |
| 1536 | 761 ns | 693 ns | 2.1 Gelem/s |

### Search Latency (768D vectors, k=10)

| Scale | EdgeVec | Target | Status |
|:------|:--------|:-------|:-------|
| 1k vectors | **380 us** | <1 ms | 2.6x under |
| 10k vectors | **938 us** | <1 ms | PASS |

### Hamming Distance (Binary Quantization)

| Operation | Time | Throughput |
|:----------|:-----|:-----------|
| 768-bit pair | 4.5 ns | 40 GiB/s |
| Batch 10k | 79 us | 127 Melem/s |

### Browser Support

| Browser | SIMD | Performance |
|:--------|:-----|:------------|
| Chrome 91+ | YES | Full speed |
| Firefox 89+ | YES | Full speed |
| Safari 16.4+ | YES | Full speed (macOS) |
| Edge 91+ | YES | Full speed |
| iOS Safari | NO | Scalar fallback |

> **Note:** iOS Safari doesn't support WASM SIMD. EdgeVec automatically uses scalar
> fallback, which is ~2x slower but still functional.

### Bundle Size

| Package | Size (gzip) | Notes |
|:--------|:------------|:------|
| edgevec | **217 KB** | SIMD enabled (541 KB uncompressed) |

[Full benchmark report ->](docs/benchmarks/2025-12-24_simd_benchmark.md)

---

## Database Features

### Binary Quantization (v0.6.0)

32x memory reduction with minimal recall loss:

```javascript
// BQ is auto-enabled for dimensions divisible by 8
const db = new EdgeVec({ dimensions: 768 });

// Raw BQ search (~85% recall, ~5x faster)
const bqResults = db.searchBQ(query, 10);

// BQ + rescore (~95% recall, ~3x faster)
const rescoredResults = db.searchBQRescored(query, 10, 5);
```

| Mode | Memory (100k × 768D) | Speed | Recall@10 |
|:-----|:---------------------|:------|:----------|
| F32 (baseline) | ~300 MB | 1x | 100% |
| BQ raw | **~10 MB** | 5x | ~85% |
| BQ + rescore(5) | **~10 MB** | 3x | ~95% |

### Metadata Filtering (v0.6.0)

Insert vectors with metadata, search with SQL-like filter expressions:

```javascript
// Insert with metadata
db.insertWithMetadata(vector, {
    category: "electronics",
    price: 299.99,
    tags: ["featured", "sale"]
});

// Search with filter
db.searchWithFilter(query, 'category = "electronics" AND price < 500', 10);
db.searchWithFilter(query, 'tags ANY ["featured"]', 10);  // Array membership

// Complex expressions
db.searchWithFilter(query,
    '(category = "electronics" OR category = "books") AND price < 100',
    10
);
```

**Operators:** `=`, `!=`, `>`, `<`, `>=`, `<=`, `AND`, `OR`, `NOT`, `ANY`

[Filter syntax documentation ->](docs/api/FILTER_SYNTAX.md)

### Memory Pressure API (v0.6.0)

Monitor and control WASM heap usage:

```javascript
const pressure = db.getMemoryPressure();
// { level: 'normal', usedBytes: 52428800, totalBytes: 268435456, usagePercent: 19.5 }

if (pressure.level === 'warning') {
    db.compact();  // Free deleted vectors
}

if (!db.canInsert()) {
    console.warn('Memory critical, inserts blocked');
}
```

### Soft Delete & Compaction

```javascript
// O(1) soft delete
db.softDelete(id);

// Check status
console.log('Live:', db.liveCount());
console.log('Deleted:', db.deletedCount());

// Reclaim space when needed
if (db.needsCompaction()) {
    const result = db.compact();
    console.log(`Removed ${result.tombstones_removed} tombstones`);
}
```

### Persistence

```javascript
// Save to IndexedDB (browser) or filesystem
await db.save("my-vector-db");

// Load existing database
const db = await EdgeVec.load("my-vector-db");
```

### Scalar Quantization

```javascript
const config = new EdgeVecConfig(768);
config.quantized = true;  // Enable SQ8 quantization

// 3.6x memory reduction: 3.03 GB -> 832 MB at 1M vectors
```

### FlatIndex (v0.9.0)

Brute-force exact nearest neighbor search for small datasets. No graph overhead, 100% recall guarantee.

```rust
use edgevec::{FlatIndex, FlatIndexConfig, DistanceMetric};

let config = FlatIndexConfig::new(768)
    .with_metric(DistanceMetric::Cosine)
    .with_capacity(10_000);
let mut index = FlatIndex::new(config);

let id = index.insert(&embedding)?;
let results = index.search(&query, 10)?;
```

### Sparse Vectors (v0.9.0)

CSR-format sparse vector storage with inverted index for fast keyword-style retrieval.

```rust
use edgevec::SparseVector;
use edgevec::sparse::{SparseStorage, SparseSearcher};

let sv = SparseVector::new(
    vec![10, 42, 999],       // term indices
    vec![0.8, 1.2, 0.3],    // term weights
    30_000,                   // vocabulary size
)?;

let mut storage = SparseStorage::new();
let id = storage.insert(&sv)?;

let searcher = SparseSearcher::new(&storage);
let results = searcher.search(&query_sv, 10);
```

### Hybrid Search (v0.9.0)

Combine dense (HNSW) and sparse retrieval with RRF or linear fusion.

```rust
use edgevec::hybrid::{HybridSearcher, HybridSearchConfig, FusionMethod};

let searcher = HybridSearcher::new(&hnsw_index, &dense_storage, &sparse_storage);

let config = HybridSearchConfig::new(
    50, 50, 10,
    FusionMethod::Rrf { k: 60 },
);

let results = searcher.search(&dense_query, &sparse_query, &config)?;
```

### BinaryFlatIndex (v0.9.0)

Native binary vector storage with Hamming distance search. 32x memory reduction.

```rust
use edgevec::BinaryFlatIndex;

let mut index = BinaryFlatIndex::new(768)?;
let id = index.insert(&binary_vector)?;
let results = index.search(&query, 10)?;
```

---

## Rust Usage

```rust
use edgevec::{HnswConfig, HnswIndex, VectorStorage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = HnswConfig::new(768);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage)?;

    // Insert
    let vector = vec![0.1; 768];
    let id = index.insert(&vector, &mut storage)?;

    // Search
    let query = vec![0.1; 768];
    let results = index.search(&query, 10, &storage)?;

    // Soft delete
    index.soft_delete(id)?;

    Ok(())
}
```

---

## Documentation

| Document | Description |
|:---------|:------------|
| [Tutorial](docs/TUTORIAL.md) | Getting started guide |
| [Filter Syntax](docs/api/FILTER_SYNTAX.md) | Complete filter expression reference |
| [Database Operations](docs/api/DATABASE_OPERATIONS.md) | CRUD operations guide |
| [FlatIndex API](docs/api/FLAT_INDEX.md) | FlatIndex reference |
| [Sparse Vectors](docs/api/SPARSE_VECTORS.md) | Sparse vector storage and search |
| [Hybrid Search](docs/api/HYBRID_SEARCH.md) | Dense + sparse fusion guide |
| [BinaryFlatIndex](docs/api/BINARY_FLAT_INDEX.md) | Binary vector index reference |
| [Performance Tuning](docs/PERFORMANCE_TUNING.md) | HNSW parameter optimization |
| [Migration Guide](docs/MIGRATION.md) | Migrating from hnswlib, FAISS, Pinecone |
| [Comparison](docs/COMPARISON.md) | When to use EdgeVec vs alternatives |

---

## Limitations

EdgeVec is designed for client-side vector search. It is **NOT** suitable for:

- **Billion-scale datasets** — Browser memory limits apply (~1GB practical limit)
- **Multi-user concurrent access** — Single-user, single-tab design
- **Distributed deployments** — Runs locally only

For these use cases, consider [Pinecone](https://pinecone.io), [Qdrant](https://qdrant.tech), or [Weaviate](https://weaviate.io).

---

## Version History

- **v0.9.0** — Sparse vectors (CSR), hybrid search (RRF + linear fusion), FlatIndex, BinaryFlatIndex (PR #7 by @marlon-costa-dc)
- **v0.8.0** — Vue 3 composables, functional filter API, SIMD Euclidean, tech debt reduction
- **v0.7.0** — SIMD acceleration (2x+ speedup), **First Community Contribution** (@jsonMartin — 8.75x Hamming)
- **v0.6.0** — Binary quantization (32x memory), metadata storage, memory pressure API
- **v0.5.4** — iOS Safari compatibility fixes
- **v0.5.3** — crates.io publishing fix (package size reduction)
- **v0.5.2** — npm TypeScript compilation fix
- **v0.5.0** — Metadata filtering with SQL-like syntax, Filter Playground demo
- **v0.4.0** — Documentation sprint, benchmark dashboard, chaos testing
- **v0.3.0** — Soft delete API, compaction, persistence format v3
- **v0.2.0** — Scalar quantization (SQ8), SIMD optimization
- **v0.1.0** — Initial release with HNSW indexing

---

## Contributors

Thank you to everyone who has contributed to EdgeVec!

| Contributor | Contribution |
|:------------|:-------------|
| [@jsonMartin](https://github.com/jsonMartin) | SIMD Hamming distance (PR #4) — 8.75x speedup |
| [@marlon-costa-dc](https://github.com/marlon-costa-dc) | BinaryFlatIndex + clippy quality fixes (PR #7, PR #8) |

---

## License

Licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE))
* MIT license ([LICENSE-MIT](./LICENSE-MIT))

at your option.

---

<div align="center">

**Built with Rust + WebAssembly**

[GitHub](https://github.com/matte1782/edgevec) |
[npm](https://www.npmjs.com/package/edgevec) |
[crates.io](https://crates.io/crates/edgevec) |
[Demos](wasm/examples/index.html)

</div>

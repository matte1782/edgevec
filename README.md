# EdgeVec

[![CI](https://github.com/matte1782/edgevec/actions/workflows/ci.yml/badge.svg)](https://github.com/matte1782/edgevec/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/edgevec.svg)](https://crates.io/crates/edgevec)
[![npm](https://img.shields.io/npm/v/edgevec.svg)](https://www.npmjs.com/package/edgevec)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/matte1782/edgevec/blob/main/LICENSE-MIT)

> **The first WASM-native vector database.**
> Filter, delete, persist — all in the browser.

EdgeVec is an embedded vector database built in Rust with first-class WebAssembly support. It brings server-grade vector database features to the browser: metadata filtering, soft delete, persistence, and sub-millisecond search.

---

## Why EdgeVec?

| Feature | EdgeVec | hnswlib-wasm | Pinecone |
|:--------|:-------:|:------------:|:--------:|
| Vector Search | Yes | Yes | Yes |
| **Metadata Filtering** | **Yes** | No | Yes |
| **SQL-like Queries** | **Yes** | No | Yes |
| **Soft Delete** | **Yes** | No | Yes |
| **Persistence** | **Yes** | No | Yes |
| Browser-native | Yes | Yes | No |
| No server required | Yes | Yes | No |
| Offline capable | Yes | Yes | No |

**EdgeVec is the only WASM vector database with filtered search.**

---

## Quick Start

```bash
npm install edgevec
```

```typescript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();

// Create a 768-dimensional index
const config = new EdgeVecConfig(768);
const db = new EdgeVec(config);

// Insert vectors with metadata
const vector = new Float32Array(768).map(() => Math.random());
const id = db.insert(vector);
// Store metadata separately for filtering
metadata[id] = { category: "books", price: 29.99, inStock: true };

// Search with filter
const query = new Float32Array(768).map(() => Math.random());
const results = db.search(query, 10);

// Filter results client-side (v0.5 pattern)
const filtered = results.filter(r =>
    metadata[r.id]?.category === "books" &&
    metadata[r.id]?.price < 50
);

// Or use the Filter API for complex expressions
import { parseFilter } from 'edgevec';
const filter = parseFilter('category = "books" AND price < 50');
```

---

## Interactive Demos

Try EdgeVec directly in your browser:

| Demo | Description |
|:-----|:------------|
| [**Filter Playground**](wasm/examples/filter-playground.html) | Interactive filter syntax explorer with live parsing |
| [**Benchmark Dashboard**](wasm/examples/benchmark-dashboard.html) | Performance comparison vs competitors |
| [**Soft Delete Demo**](wasm/examples/soft_delete.html) | Tombstone-based deletion with compaction |
| [**Main Demo**](wasm/examples/index.html) | Complete feature showcase |

```bash
# Run demos locally
git clone https://github.com/matte1782/edgevec.git
cd edgevec
python -m http.server 8080
# Open http://localhost:8080/wasm/examples/index.html
```

---

## Performance

### Search Latency (768D vectors, k=10)

| Scale | EdgeVec | Target | Status |
|:------|:--------|:-------|:-------|
| 10k vectors | **88 us** | <1 ms | 11x under |
| 50k vectors | **167 us** | <1 ms | 6x under |
| 100k vectors | **329 us** | <1 ms | 3x under |

### Competitive Comparison (10k vectors, 128D)

| Library | Search P50 | Type | Notes |
|:--------|:-----------|:-----|:------|
| **EdgeVec** | **0.20 ms** | WASM | Fastest WASM solution |
| hnswlib-node | 0.05 ms | Native C++ | Requires compilation |
| voy | 4.78 ms | WASM | k-d tree algorithm |

**EdgeVec is 24x faster than voy** for search while both are pure WASM.

### Bundle Size

| Package | Size (gzip) | Target | Status |
|:--------|:------------|:-------|:-------|
| edgevec | **227 KB** | <500 KB | 55% under |

[Full benchmarks ->](docs/benchmarks/competitive_analysis_v2.md)

---

## Database Features

### Metadata Filtering (v0.5)

EdgeVec supports SQL-like filter expressions:

```javascript
// Comparison operators
'price > 100'
'category = "electronics"'
'rating >= 4.5'

// Boolean logic
'category = "books" AND price < 50'
'brand = "Sony" OR brand = "Samsung"'
'inStock = true AND NOT discontinued = true'

// Complex expressions
'(category = "electronics" AND price < 500) OR rating >= 4.8'
```

**15 operators supported:** `=`, `!=`, `>`, `<`, `>=`, `<=`, `IN`, `NOT IN`, `CONTAINS`, `STARTS_WITH`, `ENDS_WITH`, `IS NULL`, `IS NOT NULL`, `AND`, `OR`, `NOT`

[Filter syntax documentation ->](docs/api/FILTER_SYNTAX.md)

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

- **v0.5.0** — Metadata filtering with SQL-like syntax, Filter Playground demo
- **v0.4.0** — Documentation sprint, benchmark dashboard, chaos testing
- **v0.3.0** — Soft delete API, compaction, persistence format v3
- **v0.2.0** — Scalar quantization (SQ8), SIMD optimization
- **v0.1.0** — Initial release with HNSW indexing

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

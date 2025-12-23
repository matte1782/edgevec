# EdgeVec API Reference

**Version:** EdgeVec v0.6.0
**Last Updated:** 2025-12-22

---

## Overview

EdgeVec is a high-performance embedded vector database for browser and Node.js environments. This documentation covers the complete API for both JavaScript/TypeScript and Rust usage.

## API Documentation

| Document | Description |
|:---------|:------------|
| [WASM_INDEX.md](./WASM_INDEX.md) | Main `EdgeVec` class reference for JavaScript/TypeScript |
| [TYPESCRIPT_API.md](./TYPESCRIPT_API.md) | Complete TypeScript type definitions |
| [FILTER_SYNTAX.md](./FILTER_SYNTAX.md) | Filter expression reference |
| [MEMORY.md](./MEMORY.md) | Memory management and pressure API |
| [DATABASE_OPERATIONS.md](./DATABASE_OPERATIONS.md) | CRUD operations guide |
| [ERROR_REFERENCE.md](./ERROR_REFERENCE.md) | Error codes and handling |

---

## Quick Start

### JavaScript/TypeScript

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
const results = db.searchFiltered(query, 'category = "books" AND price < 50', 10);

// Fast BQ search with rescoring (v0.6.0)
const fastResults = db.searchBQ(query, 10);

// Monitor memory pressure (v0.6.0)
const pressure = db.getMemoryPressure();
if (pressure.level === 'warning') {
    db.compact();  // Free deleted vectors
}
```

### Rust

```bash
cargo add edgevec
```

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

    Ok(())
}
```

---

## v0.6.0 Features Summary

### Binary Quantization (32x Memory Reduction)

```javascript
// BQ is auto-enabled for dimensions divisible by 8
const db = new EdgeVec({ dimensions: 768 });

// Raw BQ search (~85% recall, ~5x faster)
const bqResults = db.searchBQ(query, 10);

// BQ + rescore (~95% recall, ~3x faster)
const rescoredResults = db.searchBQRescored(query, 10, 5);
```

| Mode | Memory (100k x 768D) | Speed | Recall@10 |
|:-----|:---------------------|:------|:----------|
| F32 (baseline) | ~300 MB | 1x | 100% |
| BQ raw | **~10 MB** | 5x | ~85% |
| BQ + rescore(5) | **~10 MB** | 3x | ~95% |

### Metadata Filtering

```javascript
// Insert with metadata
db.insertWithMetadata(vector, {
    category: "electronics",
    price: 299.99,
    tags: ["featured", "sale"]
});

// Search with filter
db.searchFiltered(query, 'category = "electronics" AND price < 500', 10);
db.searchFiltered(query, 'tags ANY ["featured"]', 10);
```

**Operators:** `=`, `!=`, `>`, `<`, `>=`, `<=`, `AND`, `OR`, `NOT`, `ANY`

### Memory Pressure API

```javascript
const pressure = db.getMemoryPressure();
// { level: 'normal', usedBytes: 52428800, totalBytes: 268435456, usagePercent: 19.5 }

if (pressure.level === 'warning') {
    db.compact();
}

if (!db.canInsert()) {
    console.warn('Memory critical, inserts blocked');
}
```

---

## API Categories

### Constructor & Configuration

| Method | Description |
|:-------|:------------|
| `new EdgeVec(config)` | Create new index |
| `EdgeVec.load(name)` | Load from IndexedDB |

### Insert Methods

| Method | Description |
|:-------|:------------|
| `insert(vector)` | Insert vector, returns ID |
| `insertWithMetadata(vector, metadata)` | Insert with metadata |
| `insertBatch(vectors)` | Bulk insert |

### Search Methods

| Method | Description |
|:-------|:------------|
| `search(query, k)` | Standard F32 search |
| `searchFiltered(query, filter, k)` | Search with metadata filter |
| `searchBQ(query, k)` | Fast binary quantization search |
| `searchBQRescored(query, k, factor)` | BQ + F32 rescoring |

### Metadata Methods

| Method | Description |
|:-------|:------------|
| `getMetadata(id)` | Get vector metadata |
| `setMetadata(id, key, value)` | Set metadata field |
| `getAllMetadata(id)` | Get all metadata for vector |

### Delete Methods

| Method | Description |
|:-------|:------------|
| `softDelete(id)` | Mark as deleted |
| `isDeleted(id)` | Check deletion status |
| `compact()` | Remove deleted vectors |

### Memory Methods

| Method | Description |
|:-------|:------------|
| `getMemoryPressure()` | Get memory usage info |
| `canInsert()` | Check if inserts allowed |
| `setMemoryConfig(config)` | Configure thresholds |

### Persistence Methods

| Method | Description |
|:-------|:------------|
| `save(name)` | Save to IndexedDB |
| `createSnapshot()` | Create binary snapshot |
| `loadSnapshot(data)` | Load from snapshot |

### Utility Methods

| Method | Description |
|:-------|:------------|
| `vectorCount()` | Total vector count |
| `liveCount()` | Non-deleted count |
| `deletedCount()` | Tombstone count |
| `hasBQ()` | Check if BQ enabled |

---

## Performance Targets

| Metric | Target | Achieved |
|:-------|:-------|:---------|
| Search (10k vectors) | <1 ms | 88 us |
| Search (100k vectors) | <10 ms | 329 us |
| Insert latency | <5 ms | ~2 ms |
| Bundle size (gzip) | <500 KB | 227 KB |
| BQ memory reduction | 8-32x | 32x |
| BQ recall@10 | >0.90 | 0.95 |

---

## Browser Compatibility

| Browser | Version | Status |
|:--------|:--------|:-------|
| Chrome | 90+ | Full support |
| Firefox | 90+ | Full support |
| Safari | 15+ | Full support |
| Safari iOS | 15+ | Full support |
| Edge | 90+ | Full support |

---

## See Also

- [Tutorial](../TUTORIAL.md) - Getting started guide
- [Performance Tuning](../PERFORMANCE_TUNING.md) - HNSW parameter optimization
- [Migration Guide](../MIGRATION.md) - Migrating from other libraries
- [Troubleshooting](../TROUBLESHOOTING.md) - Common issues and solutions

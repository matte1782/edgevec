# RFC-002: Metadata Storage Requirements

**Document:** W25.5.1 — Metadata Storage Requirements Analysis
**Author:** META_ARCHITECT
**Date:** 2025-12-20
**Status:** [APPROVED]

---

## 1. Current State Analysis

### 1.1 Existing Implementation (v0.5.4)

EdgeVec already has a `MetadataStore` implementation in `src/metadata/`:

```
src/metadata/
├── mod.rs          # Module documentation and re-exports
├── types.rs        # MetadataValue enum (5 types)
├── store.rs        # MetadataStore HashMap<u32, HashMap<String, MetadataValue>>
├── error.rs        # MetadataError enum
└── validation.rs   # Key/value validation functions
```

**Current MetadataStore Architecture:**
```rust
pub struct MetadataStore {
    data: HashMap<u32, HashMap<String, MetadataValue>>,
}
```

**Current MetadataValue Types:**
- `String(String)` — Text values, max 64KB
- `Integer(i64)` — 64-bit signed integers
- `Float(f64)` — 64-bit floats (no NaN/Inf)
- `Boolean(bool)` — True/false flags
- `StringArray(Vec<String>)` — Tag arrays, max 1024 elements

**Current Limits:**
- Max 64 keys per vector
- Max 256 bytes per key name
- Max 64KB per string value
- Max 1024 elements per string array

### 1.2 Current User Pattern (v0.5.4)

```javascript
// User must manage metadata externally
const db = new EdgeVec({ dimensions: 128 });
const metadata = new Map();  // External storage

// Insert vector, then manually track metadata
const id = db.insert(vector);
metadata.set(id, { category: "books", price: 29.99 });

// Search requires manual filtering
const results = db.search(query, 10);
const filtered = results.filter(r => {
  const meta = metadata.get(r.id);
  return meta && meta.price < 50;
});
```

**Pain Points:**
1. **Two-step workflow:** Insert vector, then separately store metadata
2. **Manual synchronization:** Delete vector but forget metadata → memory leak
3. **No integrated filtering:** Filter.parse() works on external objects, not DB
4. **Persistence gap:** Save/load index, but metadata must be persisted separately
5. **WASM boundary overhead:** Passing metadata objects back and forth

---

## 2. User Pain Points (Detailed)

### 2.1 Memory Leaks on Delete

```javascript
// BUG: Metadata orphaned when vector is deleted
db.softDelete(vectorId);
// metadata.delete(vectorId); // User forgets this!
// Memory leak: metadata entry persists forever
```

**Impact:** Over time, metadata store grows with orphaned entries.

### 2.2 Inconsistent Persistence

```javascript
// Save index
await db.save("my-index");

// Save metadata separately (user must remember!)
localStorage.setItem("my-index-metadata", JSON.stringify(Object.fromEntries(metadata)));

// On load, user must reload both and pray they're in sync
```

**Impact:** Data corruption risk if versions mismatch after partial failures.

### 2.3 Filtered Search is Inefficient

```javascript
// Current: Fetch ALL results, then filter client-side
const results = db.search(query, 1000);  // Overfetch!
const filtered = results.filter(r => {
  const meta = metadata.get(r.id);
  return meta && Filter.parse('price < 50').evaluate(meta);
}).slice(0, 10);  // Then truncate
```

**Impact:**
- Wastes CPU on vectors that will be filtered out
- Higher memory usage for large result sets
- Poor latency for selective filters

### 2.4 WASM Boundary Overhead

```javascript
// Every filter evaluation crosses WASM boundary twice
for (const result of results) {
  const meta = metadata.get(result.id);  // JS object lookup
  const filterResult = Filter.parse('price < 50').evaluate(meta);  // WASM call
}
```

**Impact:** N WASM calls per search, instead of 1 integrated call.

---

## 3. Target User Pattern (v0.6.0)

### 3.1 Proposed API

```javascript
// Single-step insert with metadata
const id = db.insertWithMetadata(vector, {
  category: "books",
  price: 29.99,
  tags: ["fiction", "bestseller"]
});

// Integrated filtered search (metadata evaluated in WASM)
const results = db.searchFiltered(query, {
  filter: 'category = "books" AND price < 50',
  k: 10
});

// Automatic metadata cleanup on delete
db.softDelete(id);  // Metadata automatically marked for cleanup

// Unified persistence
await db.save("my-index");  // Includes metadata
const loaded = await EdgeVec.load("my-index");  // Metadata restored
```

### 3.2 API Method Summary

| Method | Description |
|:-------|:------------|
| `insertWithMetadata(vector, metadata)` | Insert vector + metadata atomically |
| `getMetadata(vectorId)` | Retrieve metadata for a vector |
| `updateMetadata(vectorId, metadata)` | Update metadata (merge or replace) |
| `searchFiltered(query, options)` | Search with filter evaluated in WASM |
| `save/load` | Persist and restore with metadata |

### 3.3 Backward Compatibility

The existing `insert()` method continues to work without metadata:
```javascript
const id = db.insert(vector);  // Still works, no metadata
```

---

## 4. Requirements

### 4.1 Functional Requirements

| ID | Requirement | Priority |
|:---|:------------|:---------|
| F1 | Insert vector with metadata atomically | MUST |
| F2 | Retrieve metadata by vector ID | MUST |
| F3 | Update metadata for existing vector | MUST |
| F4 | Delete metadata when vector is soft-deleted | MUST |
| F5 | Compact removes orphaned metadata | MUST |
| F6 | Search with filter expression evaluated in WASM | MUST |
| F7 | Persist metadata with index snapshot | MUST |
| F8 | Load metadata with index snapshot | MUST |
| F9 | Backward compatibility with insert() | MUST |
| F10 | Support all 5 MetadataValue types | MUST |
| F11 | Support nested filter expressions | SHOULD |
| F12 | Batch insert with metadata | SHOULD |

### 4.2 Non-Functional Requirements

| ID | Requirement | Target |
|:---|:------------|:-------|
| NF1 | Memory overhead per vector | ≤50 bytes (no metadata) |
| NF2 | Memory overhead per metadata key | ≤32 bytes + value size |
| NF3 | Filter evaluation latency | ≤1μs per vector |
| NF4 | Persistence size increase | ≤10% overhead for empty metadata |
| NF5 | WASM boundary crossings per search | ≤1 (vs current N) |

### 4.3 Constraints

| ID | Constraint | Reason |
|:---|:-----------|:-------|
| C1 | Max 64 keys per vector | Already enforced in MetadataStore |
| C2 | Max 256-byte key names | Already enforced |
| C3 | Max 64KB string values | Already enforced |
| C4 | No schema enforcement | EdgeVec is schemaless like MongoDB |
| C5 | Serialize to postcard/JSON | Must work with existing persistence |

---

## 5. Storage Constraints Analysis

### 5.1 Memory Budget

**Scenario:** 100K vectors, avg 5 metadata keys each, avg 50 bytes per key-value

```
Current (external Map):
- JavaScript Map overhead: ~50 bytes per entry [FACT: V8 implementation]
- Total: 100K × 5 × 50 = 25 MB (in JS heap, separate from WASM)

Target (integrated):
- Rust HashMap<u32, HashMap<String, MetadataValue>>
- HashMap overhead model [FACT: https://ntietz.com/blog/rust-hashmap-overhead/]:
  - Hashbrown has ~1 byte overhead per entry
  - Load factor 7/8 means ~14% average slack
  - Total overhead: ~73% of (key + value) size on average
- Per-vector overhead:
  - Outer HashMap entry: 4 bytes (u32 key) + 8 bytes (ptr) + 1 byte = ~13 bytes
  - Inner HashMap: 56 bytes struct + entries
  - 5 keys × 50 bytes = 250 bytes data × 1.73 overhead = ~433 bytes
- Total: 100K × (13 + 56 + 433) = ~50 MB (in WASM heap)

Overhead increase: ~100% more memory but unified in WASM heap
```

**Trade-off:** More WASM memory, but eliminates WASM-JS boundary overhead.

**Source:** [Rust HashMap Overhead Analysis](https://ntietz.com/blog/rust-hashmap-overhead/)

### 5.2 Persistence Size

**Current snapshot format (v0.3):**
- Header: 64 bytes
- Vectors: N × D × 4 bytes (F32) or N × (D/8) bytes (quantized)
- HNSW graph: ~50-100 bytes per vector
- Tombstones: N bits

**Target format (v0.4):**
- Same as above, plus:
- Metadata section: JSON or MessagePack serialized MetadataStore
- Estimated: 100K vectors × 5 keys × 50 bytes = 25 MB additional

### 5.3 WASM Memory Impact

| Vectors | Metadata Keys | Current (JS) | Target (WASM) | Δ |
|:--------|:--------------|:-------------|:--------------|:--|
| 10K | 0 | 0 | 0 | +0% |
| 10K | 5/vec | 2.5 MB (JS) | 5 MB (WASM) | +100% |
| 100K | 5/vec | 25 MB (JS) | 50 MB (WASM) | +100% |
| 100K | 0 | 0 | 0 | +0% |

**Note:** WASM memory overhead is ~100% higher due to Rust HashMap overhead model (~73% slack).
This is offset by eliminating WASM-JS boundary crossing overhead.

**Conclusion:** Memory increase is acceptable given the performance benefits.

---

## 6. Integration Points

### 6.1 Existing Code Affected

| Module | Change Required |
|:-------|:----------------|
| `src/hnsw/graph.rs` | Add MetadataStore field to HnswGraph |
| `src/hnsw/operations.rs` | Modify insert/delete to handle metadata |
| `src/persistence/` | Add metadata section to snapshot format |
| `src/filter/` | Already exists, needs integration with search |
| `src/wasm/` | Add WASM bindings for new methods |

### 6.2 New Files Required

| File | Purpose |
|:-----|:--------|
| `src/hnsw/metadata_integration.rs` | MetadataStore integration with HNSW |
| `src/wasm/metadata_api.rs` | WASM bindings for metadata API |

---

## 7. Success Metrics

| Metric | Current | Target |
|:-------|:--------|:-------|
| Lines of user code for filtered search | 10+ | 3 |
| WASM boundary crossings per search | N | 1 |
| Orphaned metadata after delete | Possible | Impossible |
| Persistence complexity | 2 files | 1 file |
| Filter evaluation location | JS | WASM |

---

## 8. Open Questions

1. **Schema validation:** Should EdgeVec validate metadata schema on insert?
   - Recommendation: No, stay schemaless for flexibility

2. **Null handling:** What happens when filtering on missing keys?
   - Recommendation: Missing key → filter returns false

3. **Index on metadata:** Should we support indexed metadata fields?
   - Recommendation: Defer to v0.7.0, evaluate need first

4. **Metadata versioning:** Should metadata have its own version?
   - Recommendation: Include in persistence format header

---

**Document Status:** [APPROVED]
**Next:** W25.5.2 Storage Architecture Options

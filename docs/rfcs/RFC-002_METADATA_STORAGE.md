# RFC-002: Integrated Metadata Storage

**Status:** [APPROVED]
**Author:** META_ARCHITECT
**Date:** 2025-12-20
**Target Version:** v0.6.0

---

## Summary

This RFC proposes integrating the existing `MetadataStore` into `HnswIndex` to provide atomic insert-with-metadata, automatic cleanup on delete, integrated filtered search, and unified persistence. This eliminates the user pain point of managing metadata externally.

---

## 1. Motivation

### 1.1 Current User Experience (v0.5.x)

Users must manage metadata separately from vectors:

```javascript
// External metadata management
const db = new EdgeVec({ dimensions: 128 });
const metadata = new Map();  // User-managed

// Two-step insert
const id = db.insert(vector);
metadata.set(id, { category: "books", price: 29.99 });

// Manual filtered search
const results = db.search(query, 100);  // Overfetch
const filtered = results.filter(r => {
  const meta = metadata.get(r.id);
  return meta && Filter.parse('price < 50').evaluate(meta);
}).slice(0, 10);

// User must remember to clean up
db.softDelete(id);
metadata.delete(id);  // Easy to forget!
```

### 1.2 Pain Points

| Issue | Impact |
|:------|:-------|
| Memory leaks on delete | Orphaned metadata accumulates |
| Inconsistent persistence | Index and metadata can diverge |
| Inefficient filtering | Must overfetch then filter client-side |
| WASM boundary overhead | N boundary crossings per search |
| Cognitive load | User must track two data structures |

### 1.3 Target User Experience (v0.6.0)

```javascript
// Integrated metadata
const db = new EdgeVec({ dimensions: 128 });

// Atomic insert with metadata
const id = db.insertWithMetadata(vector, {
  category: "books",
  price: 29.99,
  tags: ["fiction", "bestseller"]
});

// Integrated filtered search (evaluated in WASM)
const results = db.searchFiltered(query, {
  filter: 'category = "books" AND price < 50',
  k: 10
});

// Automatic metadata cleanup
db.softDelete(id);  // Metadata cleaned up automatically

// Unified persistence
await db.save("my-index");  // Includes metadata
```

---

## 2. Detailed Design

### 2.1 Architecture Decision

**Chosen: Option B (Sidecar Storage)**

The existing `MetadataStore` will be added as a field to `HnswIndex`:

#### 2.1.1 Thread Safety

`MetadataStore` contains `HashMap<u32, HashMap<String, MetadataValue>>`.

- `MetadataStore` is `Send + Sync` when all contained types are `Send + Sync`
- `String` and `MetadataValue` are both `Send + Sync`
- Therefore, `HnswIndex` with `MetadataStore` remains `Send + Sync`

**Concurrent modification requires external synchronization** (Mutex/RwLock at application level).
This matches the existing pattern where `HnswIndex` mutations require `&mut self`.

```rust
pub struct HnswIndex {
    // Existing fields (unchanged)
    pub config: HnswConfig,
    pub(crate) nodes: Vec<HnswNode>,
    pub(crate) neighbors: NeighborPool,
    pub(crate) entry_point: Option<NodeId>,
    pub(crate) max_layer: u8,
    pub(crate) level_mult: f32,
    rng: ChaCha8Rng,
    pub(crate) deleted_count: usize,
    compaction_threshold: f64,

    // NEW: Integrated metadata storage
    pub(crate) metadata: MetadataStore,
}
```

**Rationale:**
- Zero overhead for vectors without metadata
- No breaking changes to `HnswNode` layout
- `MetadataStore` already implemented with full test coverage
- Simple migration path

### 2.2 Existing MetadataStore

The `MetadataStore` is already implemented in `src/metadata/`:

```rust
pub struct MetadataStore {
    data: HashMap<u32, HashMap<String, MetadataValue>>,
}

pub enum MetadataValue {
    String(String),           // Max 64KB
    Integer(i64),             // 64-bit signed
    Float(f64),               // No NaN/Inf
    Boolean(bool),            // True/false
    StringArray(Vec<String>), // Max 1024 elements
}
```

**Limits:**
- 64 keys per vector
- 256 bytes per key name
- 64KB per string value
- 1024 elements per string array

### 2.3 Integration Points

| Module | Change |
|:-------|:-------|
| `src/hnsw/graph.rs` | Add `metadata: MetadataStore` field |
| `src/hnsw/operations.rs` | Add `insert_with_metadata()` method |
| `src/hnsw/operations.rs` | Modify `soft_delete()` to cleanup metadata |
| `src/hnsw/operations.rs` | Modify `compact()` to compact metadata |
| `src/hnsw/operations.rs` | Add `search_filtered()` method |
| `src/persistence/snapshot.rs` | Add metadata section to snapshot |
| `src/persistence/header.rs` | Add `HAS_METADATA` flag |
| `src/wasm/` | Add WASM bindings |

---

## 3. API Changes

### 3.1 Rust API

```rust
impl HnswIndex {
    /// Insert a vector with metadata atomically.
    ///
    /// # Arguments
    /// * `storage` - The vector storage
    /// * `vector` - The vector data
    /// * `metadata` - Key-value metadata pairs
    ///
    /// # Returns
    /// The assigned vector ID
    ///
    /// # Errors
    /// Returns `GraphError` if insert fails or metadata validation fails.
    pub fn insert_with_metadata(
        &mut self,
        storage: &mut VectorStorage,
        vector: &[f32],
        metadata: HashMap<String, MetadataValue>,
    ) -> Result<VectorId, GraphError>;

    /// Search with filter expression evaluated in Rust.
    ///
    /// # Arguments
    /// * `storage` - The vector storage
    /// * `query` - The query vector
    /// * `filter` - Filter expression string (e.g., "price < 50")
    /// * `k` - Number of results
    ///
    /// # Returns
    /// Vector of (VectorId, distance) tuples for matching vectors.
    pub fn search_filtered(
        &self,
        storage: &VectorStorage,
        query: &[f32],
        filter: &str,
        k: usize,
    ) -> Result<Vec<(VectorId, f32)>, GraphError>;

    /// Get metadata for a vector.
    pub fn get_metadata(&self, id: VectorId) -> Option<&HashMap<String, MetadataValue>>;

    /// Update metadata for a vector.
    pub fn update_metadata(
        &mut self,
        id: VectorId,
        key: &str,
        value: MetadataValue,
    ) -> Result<(), GraphError>;

    /// Delete specific metadata key.
    pub fn delete_metadata_key(&mut self, id: VectorId, key: &str) -> Result<bool, GraphError>;
}
```

### 3.2 Filtering Algorithm

**Chosen: Post-filtering with adaptive overfetch**

There are three broad strategies to combine filtering with ANN search:
1. **Pre-filtering:** Filter before HNSW traversal — can break graph connectivity, lower recall
2. **Post-filtering:** Filter after HNSW traversal — higher recall but wasteful for selective filters
3. **In-algorithm (ACORN):** Filter during traversal — best but most complex

**Selected Approach: Post-filtering with adaptive overfetch**

```
Algorithm: search_filtered(query, filter, k)
1. Parse filter expression using existing Filter module
2. Estimate selectivity from filter complexity:
   - Simple equality: ~10% selectivity
   - Range: ~30% selectivity
   - AND compound: product of individual selectivities
   - OR compound: sum - product
   - Default (unknown): 50%
3. Calculate overfetch factor = min(10, max(2, 1 / selectivity))
4. Execute HNSW search for k * overfetch_factor candidates
5. For each candidate:
   a. Lookup metadata from MetadataStore (O(1) HashMap access)
   b. Evaluate filter using Filter::evaluate()
   c. If passes, add to results
6. Return top-k passing filter (sorted by distance)
```

**Rationale:**
- Simplest to implement for v0.6.0
- MetadataStore already exists with O(1) lookup
- Filter module already exists with full expression evaluation
- In-algorithm filtering (ACORN) deferred to v0.7.0 if needed

**Performance Note:**
- Acceptable for filter selectivity > 10%
- For highly selective filters (< 10%), may return fewer than k results
- Future optimization: ACORN integration per [Elasticsearch Labs research](https://www.elastic.co/search-labs/blog/filtered-hnsw-knn-search)

**Source:** [Elasticsearch Labs — Filtered HNSW in Lucene](https://www.elastic.co/search-labs/blog/filtered-hnsw-knn-search)

### 3.3 WASM/JavaScript API

```typescript
interface EdgeVec {
  // Existing methods (unchanged)
  insert(vector: Float32Array): number;
  search(query: Float32Array, k: number): SearchResult[];
  softDelete(id: number): boolean;
  compact(): CompactionResult;

  // NEW: Metadata-aware methods
  insertWithMetadata(vector: Float32Array, metadata: Record<string, MetadataValue>): number;
  searchFiltered(query: Float32Array, options: FilteredSearchOptions): SearchResult[];
  getMetadata(id: number): Record<string, MetadataValue> | null;
  updateMetadata(id: number, key: string, value: MetadataValue): void;
  deleteMetadataKey(id: number, key: string): boolean;
}

interface FilteredSearchOptions {
  filter: string;  // Filter expression
  k: number;       // Number of results
}

type MetadataValue = string | number | boolean | string[];
```

### 3.4 Backward Compatibility

| Method | v0.5.x | v0.6.0 |
|:-------|:-------|:-------|
| `insert()` | Works | Works (no metadata) |
| `search()` | Works | Works (no filter) |
| `softDelete()` | Works | Works + cleans metadata |
| `save()/load()` | Works | Works + includes metadata |

Existing code continues to work without modification.

---

## 4. Memory Impact

### 4.1 Per-Vector Overhead

| Scenario | Overhead | Notes |
|:---------|:---------|:------|
| No metadata | 0 bytes | HashMap entry not created |
| Empty metadata | ~69 bytes | Outer HashMap entry (13) + inner struct (56) |
| 5 keys, 50B each | ~433 bytes | 250B data × 1.73 overhead [FACT: ntietz.com] |
| 10 keys, 100B each | ~1.7 KB | 1000B data × 1.73 overhead |

### 4.2 Total Memory (100K vectors)

| Config | v0.5.x (external) | v0.6.0 (integrated) | Δ |
|:-------|:------------------|:--------------------|:--|
| No metadata | 0 | 0 | 0% |
| 5 keys/vec, 50B | 25 MB (JS heap) | 43 MB (WASM heap) | +72% |
| 10 keys/vec, 100B | 100 MB (JS heap) | 170 MB (WASM heap) | +70% |

**Trade-off:** Higher WASM memory, but eliminates:
- WASM-JS boundary crossing overhead
- JavaScript Map overhead
- GC pressure on JS side

### 4.3 WASM Memory Limits

| Platform | Practical Limit | Safe Limit | Max Vectors (5 keys) |
|:---------|:----------------|:-----------|:---------------------|
| Desktop Chrome | 2-4 GB | 1 GB | 3M+ |
| iOS Safari | 256-512 MB | 100 MB | 300K |
| Android Chrome | ~300 MB | 100 MB | 300K |

---

## 5. Persistence Format

### 5.1 Format Version

Bump from v0.3 to v0.4:
- Add `HAS_METADATA` flag (bit 2)
- Add metadata section after deleted bitvec

### 5.2 Layout (v0.4)

```
+0x00: FileHeader (64 bytes)
  version_minor: 4 (was 3)
  flags: |= 0x04 if metadata present
+0x40: Vector data (N * D * 4 bytes)
+index_offset: HNSW data
  +0x00: HnswNodes (N * 16 bytes)
  +nodes_end: Neighbors pool (variable)
+tombstone_offset: Deleted bitvec (ceil(N/8) bytes)
+after bitvec: Metadata section (if HAS_METADATA)
  +0x00: MetadataSectionHeader (16 bytes)
    magic: "META"
    version: 1
    format: 1 (Postcard) or 2 (JSON)
    size: u32
    crc: u32
  +0x10: Serialized MetadataStore (size bytes)
```

### 5.3 Serialization

**Primary format:** Postcard (binary)
- Compact (~2-3x smaller than JSON)
- Fast serialization [HYPOTHESIS — ~50 MB/s on WASM, needs benchmarking]
- Native serde support

### 5.4 Migration

| From | To | Strategy |
|:-----|:---|:---------|
| v0.3 | v0.4 | Auto-upgrade, empty MetadataStore |
| v0.4 | v0.3 | Not supported (version check fails) |

---

## 6. Implementation Plan

### Phase 1: Core Integration (v0.6.0-alpha.1)

1. Add `metadata` field to `HnswIndex`
2. Implement `insert_with_metadata()`
3. Modify `soft_delete()` to cleanup metadata
4. Modify `compact()` to compact metadata
5. Add unit tests

**Estimated effort:** 8 hours

### Phase 2: Filtered Search (v0.6.0-alpha.2)

1. Implement `search_filtered()` in Rust
2. Integrate with existing Filter evaluation
3. Add early-exit optimization for selective filters
4. Add integration tests

**Estimated effort:** 6 hours

### Phase 3: Persistence (v0.6.0-alpha.3)

1. Add `MetadataSectionHeader` struct
2. Implement metadata serialization (Postcard)
3. Update `write_snapshot()` for v0.4
4. Update `read_snapshot()` for v0.4
5. Add v0.3 → v0.4 migration tests

**Estimated effort:** 8 hours

### Phase 4: WASM API (v0.6.0-beta.1)

1. Add `insertWithMetadata()` binding
2. Add `searchFiltered()` binding
3. Add `getMetadata()` binding
4. Update TypeScript types
5. Add WASM integration tests

**Estimated effort:** 6 hours

### Phase 5: Documentation & Release (v0.6.0)

1. Update README with new API
2. Add migration guide
3. Update API documentation
4. Performance benchmarks
5. Release

**Estimated effort:** 4 hours

**Total:** ~32 hours

---

## 7. Alternatives Considered

### 7.0 Industry Comparison

EdgeVec's sidecar metadata approach aligns with industry patterns:

| Database | Metadata Storage | Filter Strategy | Notes |
|:---------|:-----------------|:----------------|:------|
| **Qdrant** | JSON payload attached to points | Post-filter with indexed fields | [Source](https://qdrant.tech/benchmarks/filtered-search-intro/) |
| **Weaviate** | Schema-defined properties | Pre-filter with inverted index | [Source](https://weaviate.io/developers/weaviate/concepts/filtering) |
| **Pinecone** | JSON metadata per vector | Post-filter | Serverless, metadata attached |
| **EdgeVec v0.6.0** | `HashMap<u32, HashMap<String, MetadataValue>>` | Post-filter with adaptive overfetch | Sidecar pattern, schemaless |

**Key Insight:** The sidecar/payload pattern is industry-standard. EdgeVec's approach matches Qdrant and Pinecone, with schemaless flexibility like MongoDB. In-algorithm filtering (ACORN) is deferred to v0.7.0 pending performance validation.

### 7.1 Option A: Inline Storage

Embed metadata pointer in `HnswNode` (+8 bytes per node).

**Rejected because:**
- Breaking change to node layout
- All nodes pay overhead even without metadata
- Pool fragmentation requires defragmentation logic

### 7.2 Option C: Hybrid Hot/Cold

Hot fields inline, cold in HashMap.

**Rejected because:**
- Maximum complexity
- Requires schema prediction
- Two serialization systems

### 7.3 Keep External

Continue with external metadata management.

**Rejected because:**
- Doesn't solve user pain points
- Memory leaks on delete remain possible
- Persistence remains user's responsibility

---

## 8. Open Questions

### 8.1 Resolved

| Question | Decision |
|:---------|:---------|
| Storage architecture? | Sidecar (Option B) |
| Serialization format? | Postcard primary, JSON fallback |
| Persistence format version? | v0.4 |

### 8.2 Deferred to v0.7.0

| Question | Notes |
|:---------|:------|
| Metadata indexing? | Evaluate need based on usage |
| Compression? | Add zstd if metadata size is concern |
| Streaming write? | Keep atomic for now |

---

## 9. Success Metrics

| Metric | v0.5.x | v0.6.0 Target |
|:-------|:-------|:--------------|
| Lines of user code for filtered search | 10+ | 3 |
| WASM boundary crossings per search | N | 1 |
| Orphaned metadata after delete | Possible | Impossible |
| Persistence complexity | 2 files | 1 file |
| Filter evaluation location | JS | WASM |

---

## 10. References

- RFC-002_REQUIREMENTS.md — Requirements analysis
- RFC-002_ARCHITECTURE_OPTIONS.md — Architecture evaluation
- RFC-002_PERSISTENCE_FORMAT.md — Persistence format design
- src/metadata/ — Existing MetadataStore implementation
- src/filter/ — Existing Filter implementation

---

**Document Status:** [APPROVED]
**Review Required:** HOSTILE_REVIEWER
**Target Release:** v0.6.0


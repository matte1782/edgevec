# RFC-002: Storage Architecture Options

**Document:** W25.5.2 — Storage Architecture Options Analysis
**Author:** META_ARCHITECT
**Date:** 2025-12-20
**Status:** [APPROVED]

---

## 1. Options Overview

This document evaluates three storage architectures for integrating metadata with the HNSW index.

| Option | Description | Integration Level |
|:-------|:------------|:------------------|
| **A: Inline** | Metadata pointer embedded in HnswNode | Deep |
| **B: Sidecar** | Separate MetadataStore alongside HnswIndex | Shallow |
| **C: Hybrid** | Hot metadata inline, cold in sidecar | Medium |

---

## 2. Option A: Inline Storage

### 2.1 Design

Embed metadata directly in or adjacent to `HnswNode`:

```rust
// Current HnswNode: 16 bytes
#[repr(C)]
pub struct HnswNode {
    pub vector_id: VectorId,       // 8 bytes
    pub neighbor_offset: u32,      // 4 bytes
    pub neighbor_len: u16,         // 2 bytes
    pub max_layer: u8,             // 1 byte
    pub deleted: u8,               // 1 byte
}

// Option A: Extended HnswNode: 24 bytes (+8)
#[repr(C)]
pub struct HnswNodeV2 {
    pub vector_id: VectorId,       // 8 bytes
    pub neighbor_offset: u32,      // 4 bytes
    pub neighbor_len: u16,         // 2 bytes
    pub max_layer: u8,             // 1 byte
    pub deleted: u8,               // 1 byte
    pub metadata_offset: u32,      // 4 bytes (offset into metadata pool)
    pub metadata_len: u32,         // 4 bytes (serialized size in bytes)
}
```

### 2.2 Metadata Pool Layout

```
+------------------+
| Metadata Pool    |
+------------------+
| Entry 0 (var)    | <- offset 0
| Entry 1 (var)    | <- offset X
| Entry 2 (var)    | <- offset Y
| ...              |
+------------------+

Each entry: postcard-serialized HashMap<String, MetadataValue>
```

### 2.3 Evaluation

| Criterion | Score | Notes |
|:----------|:------|:------|
| **Memory Overhead** | -2 | +8 bytes per node (50% increase) |
| **Cache Locality** | +2 | Metadata offset accessed with node |
| **Persistence Complexity** | -1 | Must maintain offset consistency |
| **Filter Performance** | +2 | Direct offset lookup, no HashMap |
| **Migration Path** | -2 | Breaking change to HnswNode layout |
| **Fragmentation Risk** | -1 | Variable-size entries, needs compaction |

**Total Score: -2**

### 2.4 Pros/Cons

**Pros:**
- Best cache locality during filtered search
- No separate hash lookup during filter evaluation
- Metadata naturally co-located with node data

**Cons:**
- Breaking change to node layout (v0.4 format required)
- All nodes pay the 8-byte overhead even without metadata
- Pool fragmentation requires defragmentation logic
- Complex offset management during updates

---

## 3. Option B: Sidecar Storage

### 3.1 Design

Keep existing `MetadataStore` as a separate field in `HnswIndex`:

```rust
pub struct HnswIndex {
    pub config: HnswConfig,
    pub(crate) nodes: Vec<HnswNode>,      // Unchanged
    pub(crate) neighbors: NeighborPool,
    pub(crate) entry_point: Option<NodeId>,
    pub(crate) max_layer: u8,
    pub(crate) level_mult: f32,
    rng: ChaCha8Rng,
    pub(crate) deleted_count: usize,
    compaction_threshold: f64,

    // NEW: Sidecar metadata storage
    pub(crate) metadata: MetadataStore,   // +8 bytes (Box pointer)
}
```

### 3.2 Integration

```rust
impl HnswIndex {
    pub fn insert_with_metadata(
        &mut self,
        storage: &mut VectorStorage,
        vector: &[f32],
        metadata: HashMap<String, MetadataValue>,
    ) -> Result<VectorId, GraphError> {
        // Validate all metadata first (fail-fast before any mutations)
        for (key, value) in &metadata {
            self.metadata.validate_key(key)
                .map_err(|e| GraphError::Storage(e.to_string()))?;
            self.metadata.validate_value(value)
                .map_err(|e| GraphError::Storage(e.to_string()))?;
        }

        let id = self.insert(storage, vector)?;

        // Insert all metadata using insert_all for atomicity
        // If this fails, the vector is orphaned but will be cleaned up on compact()
        if let Err(e) = self.metadata.insert_all(id.0 as u32, metadata) {
            // Rollback: mark vector as deleted
            self.soft_delete_internal(id);
            return Err(GraphError::Storage(e.to_string()));
        }

        Ok(id)
    }

    pub fn soft_delete(&mut self, id: VectorId) -> bool {
        if self.soft_delete_internal(id) {
            // Automatically cleanup metadata
            self.metadata.delete_all(id.0 as u32);
            true
        } else {
            false
        }
    }
}
```

### 3.3 Evaluation

| Criterion | Score | Notes |
|:----------|:------|:------|
| **Memory Overhead** | +2 | 0 bytes for nodes without metadata |
| **Cache Locality** | -1 | Separate HashMap lookup per filter |
| **Persistence Complexity** | +1 | Simple: serialize MetadataStore separately |
| **Filter Performance** | 0 | HashMap lookup is O(1) but cache-cold |
| **Migration Path** | +2 | No breaking changes to HnswNode |
| **Fragmentation Risk** | +1 | HashMap handles its own memory |

**Total Score: +5**

### 3.4 Pros/Cons

**Pros:**
- Zero overhead for vectors without metadata
- Non-breaking: HnswNode layout unchanged
- Existing MetadataStore is fully implemented and tested
- Simple integration: just add a field
- Easy to serialize independently

**Cons:**
- Extra HashMap lookup per filter evaluation
- Metadata not co-located with nodes (cache miss)
- Slightly higher latency for filtered search

---

## 4. Option C: Hybrid Storage

### 4.1 Design

Split metadata into "hot" (frequently filtered) and "cold" (rarely accessed):

```rust
#[repr(C)]
pub struct HnswNodeV2 {
    // ... existing fields ...

    // Hot metadata: single u64 for common filter patterns
    pub hot_metadata: u64,  // +8 bytes
    // Interpretation: bits 0-31 = category_id, bits 32-63 = flags
}

pub struct HnswIndex {
    // ... existing fields ...

    // Cold metadata: full HashMap for infrequent access
    pub(crate) cold_metadata: MetadataStore,
}
```

### 4.2 Hot Metadata Schema

```rust
// User defines hot fields at index creation
pub struct HotMetadataSchema {
    fields: Vec<HotField>,
}

pub enum HotField {
    CategoryId,       // 32-bit enum/id
    PriceRange,       // 16-bit bucket (0-65535)
    Flags(u8),        // 8-bit flags
    Timestamp(u32),   // 32-bit unix seconds
}
```

### 4.3 Evaluation

| Criterion | Score | Notes |
|:----------|:------|:------|
| **Memory Overhead** | -1 | +8 bytes per node for hot fields |
| **Cache Locality** | +1 | Hot fields inline, cold separate |
| **Persistence Complexity** | -2 | Two systems to serialize |
| **Filter Performance** | +1 | Fast for hot fields, HashMap for cold |
| **Migration Path** | -2 | Breaking node change, schema migration |
| **Fragmentation Risk** | 0 | Mixed approach |

**Total Score: -3**

### 4.4 Pros/Cons

**Pros:**
- Optimal performance for pre-defined hot filters
- Flexible: any field can be hot or cold
- Best of both worlds for predictable workloads

**Cons:**
- Most complex implementation
- Requires upfront schema design
- Two serialization systems
- Breaking change to node layout
- User must predict which fields are "hot"

---

## 5. Comparison Matrix

| Criterion | A: Inline | B: Sidecar | C: Hybrid |
|:----------|:----------|:-----------|:----------|
| **Memory Overhead** | -2 | **+2** | -1 |
| **Cache Locality** | **+2** | -1 | +1 |
| **Persistence Complexity** | -1 | **+1** | -2 |
| **Filter Performance** | **+2** | 0 | +1 |
| **Migration Path** | -2 | **+2** | -2 |
| **Fragmentation Risk** | -1 | **+1** | 0 |
| **Total** | **-2** | **+5** | **-3** |

---

## 6. Performance Analysis

### 6.1 Filter Evaluation Cost

**Option A (Inline):** [HYPOTHESIS — needs benchmarking]
```
Cost = offset_read (1 cache line) + deserialize (variable)
     = ~5ns + ~50ns = ~55ns per vector
```

**Option B (Sidecar):** [HYPOTHESIS — needs benchmarking]
```
Cost = HashMap::get (hash + lookup) + compare
     = ~20ns (hash) + ~10ns (lookup) + ~5ns (compare) = ~35ns per vector
```

**Option C (Hybrid - Hot):** [HYPOTHESIS — needs benchmarking]
```
Cost = bit_read (same cache line as node)
     = ~2ns per vector
```

**Verification plan:** Benchmark in v0.6.0-alpha.2 phase

### 6.2 Filtered Search Latency (100K vectors, 10% selectivity)

[HYPOTHESIS — theoretical estimates, needs benchmarking]

| Option | Filter Cost | Total Candidates | Latency Impact |
|:-------|:------------|:-----------------|:---------------|
| A: Inline | 55ns | 10K | +550us |
| B: Sidecar | 35ns | 10K | +350us |
| C: Hot | 2ns | 10K | +20us |
| C: Cold | 35ns | 10K | +350us |

**Note:** These are theoretical costs. Actual performance depends on:
- Cache state (hot vs cold)
- Metadata complexity (number of keys)
- Filter complexity (number of conditions)

### 6.3 Memory Impact (100K vectors)

| Option | Base (no meta) | With 5 keys/vec | Δ |
|:-------|:---------------|:----------------|:--|
| A: Inline | 1.6 MB nodes | 4.6 MB (+offsets+pool) | +188% |
| B: Sidecar | 1.6 MB nodes | 50 MB (MetadataStore) | +3025% |
| C: Hybrid | 2.4 MB nodes | 51 MB (cold) | +2025% |

**HashMap Overhead Model** [FACT: [Rust HashMap Overhead Analysis](https://ntietz.com/blog/rust-hashmap-overhead/)]:
- Hashbrown (Rust's HashMap) has ~1 byte overhead per entry
- Load factor 7/8 means ~14% average slack
- Total overhead: ~73% of (key + value) size on average
- For 5 keys × 50 bytes = 250 bytes data → ~433 bytes total per vector

**Analysis:**
- Option A has best memory efficiency due to pool layout
- Option B uses HashMap with ~73% overhead on average (not ~50 bytes/entry as previously claimed)
- Option C has worst memory: overhead of both A and B

---

## 7. Recommendation

### 7.1 Primary Recommendation: Option B (Sidecar)

**Rationale:**

1. **Lowest Risk:** No breaking changes to HnswNode
2. **Already Implemented:** MetadataStore is complete with full test coverage
3. **Migration Path:** Simple - just add field to HnswIndex
4. **Zero Overhead:** Nodes without metadata pay nothing
5. **Performance Acceptable:** ~35ns per filter evaluation is acceptable for WASM

### 7.2 Why Not Option A?

- Breaking change to node layout is high risk
- Pool fragmentation adds complexity
- Memory overhead affects ALL nodes, even without metadata
- Benefit (cache locality) is marginal for WASM where memory access is already fast

### 7.3 Why Not Option C?

- Maximum complexity for marginal benefit
- Requires schema prediction by user
- Two serialization systems = two bug surfaces
- Hot field selection is hard to get right

### 7.4 Future Optimization Path

If Option B proves too slow (unlikely), we can add Option C's hot field optimization **additively** without breaking changes:

```rust
// v0.7.0: Add optional hot metadata
#[repr(C)]
pub struct HnswNodeV3 {
    // ... existing V1 fields ...

    // Optional hot metadata (only if enabled at construction)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hot_bits: Option<u64>,
}
```

This keeps Option B as the stable base while allowing future optimization.

---

## 8. Integration Plan for Option B

### 8.1 Phase 1: Core Integration

1. Add `metadata: MetadataStore` field to `HnswIndex`
2. Add `insert_with_metadata()` method
3. Modify `soft_delete()` to cleanup metadata
4. Modify `compact()` to compact metadata

### 8.2 Phase 2: Filtered Search

1. Add `search_filtered()` method to HnswIndex
2. Integrate Filter evaluation with MetadataStore lookup
3. Early-exit optimization for selective filters

### 8.3 Phase 3: Persistence

1. Extend snapshot format to include MetadataStore
2. Add metadata section after deleted bitvec
3. Update header to v0.4

### 8.4 Phase 4: WASM API

1. Add `insertWithMetadata(vector, metadata)` binding
2. Add `searchFiltered(query, filter, k)` binding
3. Add `getMetadata(id)` binding

---

## 9. Persistence Format Changes

### 9.1 Current v0.3 Layout

```
+0x00: FileHeader (64 bytes)
  +0x00: magic "EVEC" (4 bytes)
  +0x04: version 0.3 (2 bytes)
  +0x06: flags (2 bytes)
  +0x08: vector_count (8 bytes)
  +0x10: index_offset (8 bytes)
  +0x18: metadata_offset (8 bytes) -- currently: deleted bitvec offset
  +0x20: rng_seed (8 bytes)
  +0x28: dimensions (4 bytes)
  +0x2C: header_crc (4 bytes)
  +0x30: hnsw_m (4 bytes)
  +0x34: hnsw_m0 (4 bytes)
  +0x38: data_crc (4 bytes)
  +0x3C: deleted_count (4 bytes)
+0x40: Vector data (N * D * 4 bytes)
+index_offset: HnswNodes (N * 16 bytes)
+after nodes: Neighbors pool (variable)
+metadata_offset: Deleted bitvec (N/8 bytes)
```

### 9.2 Proposed v0.4 Layout

```
+0x00: FileHeader (64 bytes)
  +0x00: magic "EVEC" (4 bytes)
  +0x04: version 0.4 (2 bytes)  -- bumped
  +0x06: flags (2 bytes)
  +0x08: vector_count (8 bytes)
  +0x10: index_offset (8 bytes)
  +0x18: tombstone_offset (8 bytes)  -- renamed from metadata_offset
  +0x20: rng_seed (8 bytes)
  +0x28: dimensions (4 bytes)
  +0x2C: header_crc (4 bytes)
  +0x30: hnsw_m (4 bytes)
  +0x34: hnsw_m0 (4 bytes)
  +0x38: data_crc (4 bytes)
  +0x3C: deleted_count (4 bytes)
+0x40: Vector data (N * D * 4 bytes)
+index_offset: HnswNodes (N * 16 bytes)
+after nodes: Neighbors pool (variable)
+tombstone_offset: Deleted bitvec (N/8 bytes)
+after bitvec: MetadataStore (postcard serialized)
+after metadata: metadata_crc (4 bytes)
```

### 9.3 Header Extension (If Needed)

If we need more fields, extend header to 128 bytes:

```rust
// Extended header for v0.4+ (128 bytes)
#[repr(C)]
pub struct FileHeaderV4 {
    // First 64 bytes: same as v0.3
    pub base: FileHeader,

    // Extension (64 bytes)
    pub metadata_store_offset: u64,  // +64: offset to MetadataStore
    pub metadata_store_size: u64,    // +72: size of MetadataStore
    pub metadata_format: u8,         // +80: 0=None, 1=Postcard, 2=JSON
    pub reserved: [u8; 55],          // +81: reserved for future
}
```

---

## 10. Open Questions

1. **Metadata CRC:** Should MetadataStore have its own CRC, or is data_crc sufficient?
   - Recommendation: Separate CRC for partial load support

2. **Streaming Load:** Should metadata be loadable independently of vectors?
   - Recommendation: Yes, for large indices where metadata-only queries are common

3. **Metadata Versioning:** Should MetadataStore have its own version?
   - Recommendation: Yes, embed in serialization format

---

**Document Status:** [APPROVED]
**Next:** W25.5.3 Persistence Format Design


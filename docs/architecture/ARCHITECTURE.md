# EdgeVec Architecture v1.7

**Date:** 2025-12-10
**Author:** META_ARCHITECT
**Status:** [APPROVED]

---

## 0. DESIGN DECISIONS

### Requirements Extraction

| ID | Requirement | Source | Priority |
|:---|:------------|:-------|:---------|
| R1 | HNSW indexing with O(log n) search | 10_HOSTILE_GATE.md | CRITICAL |
| R2 | WASM-first design (pure Rust) | README.md, ASSET_FIT_REPORT.md | CRITICAL |
| R3 | Persistent storage (browser + native) | 10_HOSTILE_GATE.md | HIGH |
| R4 | <10ms search for 100k vectors (P99) | .cursorrules | CRITICAL |
| R5 | <100 bytes/vector memory budget | .cursorrules | HIGH |
| R6 | <500KB gzipped bundle | README.md | MEDIUM |
| R7 | Crash recovery (WAL) | 10_HOSTILE_GATE.md | HIGH |
| R8 | Deterministic Replay | Hostile Review | CRITICAL |
| R9 | Scalar Quantization (SQ8) | W6.1 | CRITICAL |

### Design Decisions

| Decision | Options Considered | Choice | Rationale |
|:---------|:-------------------|:-------|:----------|
| D1: Index Algorithm | HNSW, IVF, Annoy | **HNSW** | O(log n) search, proven in production |
| D2: Storage Backend | Custom, sled, redb | **Custom** | WASM compat, career capital |
| D3: Serialization | bincode, postcard, custom | **postcard** | WASM-friendly, no_std compatible |
| D4: Neighbor Storage | Raw `Vec<u32>`, Compressed | **Compressed (VByte)** | Essential to meet memory budget |
| D5: Memory Allocation | Vec, Arena, Pool | **Pool + Arena** | Predictable allocation, WASM-friendly |
| D6: Vector Compression | f32, f16, u8 (Scalar) | **u8 (Scalar Quant)** | Required for 1M vector scale in <1GB |
| D7: Quantization Scope | Global, Per-Vector, Product | **Global** | Simplicity and speed for v1 |

### Tradeoffs Accepted

| Tradeoff | What We Gain | What We Lose | Risk Level |
|:---------|:-------------|:-------------|:-----------|
| T1: Compressed Neighbors | 3.5x less RAM usage | CPU overhead on access | LOW (SIMD helps) |
| T2: Custom storage vs sled | WASM compatibility, control | Development time | MEDIUM |
| T3: HNSW vs brute force | O(log n) search | Insert complexity | LOW |
| T4: No multithreading initially | Simpler WASM | Slower bulk ops | LOW |
| T5: Lossy Quantization (SQ8) | 4x storage density | Precision loss | MEDIUM (offset by re-ranking optionality) |

---

## 1. System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              EdgeVec Architecture                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        PUBLIC API LAYER                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │   │
│  │  │  insert()   │  │  search()   │  │  delete()   │                  │   │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                  │   │
│  └─────────┼────────────────┼────────────────┼─────────────────────────┘   │
│            │                │                │                              │
│  ┌─────────▼────────────────▼────────────────▼─────────────────────────┐   │
│  │                        INDEX LAYER (HNSW)                            │   │
│  │  ┌─────────────────────────────────────────────────────────────┐    │   │
│  │  │  HnswIndex                                                   │    │   │
│  │  │  ├── nodes: Vec<HnswNode>                                   │    │   │
│  │  │  ├── neighbors: NeighborPool (Compressed)                   │    │   │
│  │  │  ├── entry_point: NodeId                                    │    │   │
│  │  │  ├── config: HnswConfig                                     │    │   │
│  │  │  └── rng: DeterministicRng                                  │    │   │
│  │  └─────────────────────────────────────────────────────────────┘    │   │
│  └──────────────────────────┬──────────────────────────────────────────┘   │
│                             │                                               │
│  ┌──────────────────────────▼──────────────────────────────────────────┐   │
│  │                        VECTOR LAYER                                  │   │
│  │  ┌─────────────────────────────────────────────────────────────┐    │   │
│  │  │  VectorStorage (Quantized u8)                               │    │   │
│  │  │  ├── vectors: Arena<u8>                                     │    │   │
│  │  │  ├── metadata: MetadataStore                                │    │   │
│  │  │  ├── id_map: IdAllocator                                    │    │   │
│  │  │  └── quantizer: QuantizerConfig                             │    │   │
│  │  └─────────────────────────────────────────────────────────────┘    │   │
│  └──────────────────────────┬──────────────────────────────────────────┘   │
│                             │                                               │
│  ┌──────────────────────────▼──────────────────────────────────────────┐   │
│  │                        PERSISTENCE LAYER                             │   │
│  │  ┌───────────────────┐  ┌───────────────────┐  ┌─────────────────┐  │   │
│  │  │  WriteAheadLog    │  │  SnapshotManager  │  │  StorageBackend │  │   │
│  │  │  (WAL)            │  │                   │  │  (trait)        │  │   │
│  │  └─────────┬─────────┘  └─────────┬─────────┘  └────────┬────────┘  │   │
│  │            │                      │                     │           │   │
│  │            ▼                      ▼                     ▼           │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │  Platform Adapters                                            │  │   │
│  │  │  ├── IndexedDbBackend (WASM/Browser)                         │  │   │
│  │  │  ├── FileBackend (Native)                                    │  │   │
│  │  │  └── MemoryBackend (Testing)                                 │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Component Breakdown

### 2.1 Public API Layer

**Purpose:** Provide ergonomic, type-safe interface for vector operations.

**Inputs:**
- User vectors (`&[f32]`)
- Search queries
- Configuration

**Outputs:**
- Vector IDs (`VectorId`)
- Search results (`Vec<SearchResult>`)
- Operation status

**Invariants:**
- [INV-API-1] All public methods are FFI-safe (no panics across WASM boundary)
- [INV-API-2] Vector dimensions are validated on insert (must match index config)
- [INV-API-3] Results are always sorted by distance (ascending)

```rust
/// Primary entry point for EdgeVec operations.
/// 
/// # WASM Safety
/// All public methods return `Result<T, EdgeVecError>` to avoid panics.
pub struct EdgeVec {
    index: HnswIndex,
    storage: VectorStorage,
    wal: WriteAheadLog,
    config: EdgeVecConfig,
}
```

### 2.2 Index Layer (HNSW)

**Purpose:** Provide O(log n) approximate nearest neighbor search.

**Inputs:**
- Vectors to index
- Search queries with k parameter

**Outputs:**
- Candidate neighbors for insertion
- Search results

**Invariants:**
- [INV-HNSW-1] Graph is always navigable (no orphan nodes)
- [INV-HNSW-2] Entry point is always valid
- [INV-HNSW-3] Layer 0 contains all vectors
- [INV-HNSW-4] Each node has at most `M` neighbors per layer
- [INV-HNSW-5] Deterministic behavior given same seed

**Algorithm Reference:** [FACT] HNSW as described in Malkov & Yashunin 2018 (arXiv:1603.09320)

```rust
/// Hierarchical Navigable Small World graph index.
/// 
/// # Memory Budget
/// Optimized for <100 bytes/vector overhead using VByte neighbor compression.
pub struct HnswIndex {
    /// Node metadata (vector ID, level, offset into neighbor pool)
    nodes: Vec<HnswNode>,
    /// Compressed neighbor lists (VByte encoded deltas)
    neighbors: NeighborPool,
    /// Entry point for search (highest layer node)
    entry_point: Option<NodeId>,
    /// Configuration parameters
    config: HnswConfig,
    /// Level probability multiplier (1/ln(M))
    level_mult: f32,
    /// Deterministic RNG state
    rng: DeterministicRng,
}

/// Configuration for HNSW index.
pub struct HnswConfig {
    /// Maximum connections per node (default: 16)
    pub m: usize,
    /// Maximum connections at layer 0 (default: 32, typically 2*M)
    pub m0: usize,
    /// Size of dynamic candidate list during construction (default: 200)
    pub ef_construction: usize,
    /// Size of dynamic candidate list during search (default: 50)
    pub ef_search: usize,
    /// Vector dimensionality (must be known at construction)
    pub dimensions: u32,
}
```

### 2.3 Vector Layer

**Purpose:** Store and retrieve vector data efficiently (Quantized u8).

**Inputs:**
- Raw vector data (`&[f32]`) -> Quantized on insert
- Quantized vector data (`&[u8]`) -> Direct insert
- Vector IDs

**Outputs:**
- Quantized vector data by ID
- Metadata by ID

**Invariants:**
- [INV-VEC-1] All vectors have identical dimensionality
- [INV-VEC-2] Vector IDs are unique and never reused
- [INV-VEC-3] Deleted vectors are tombstoned, not immediately freed
- [INV-VEC-4] Quantization is lossy; original f32 data is NOT stored by default (Hybrid Mode optional later)

```rust
/// Arena-based vector storage for cache-friendly access.
/// 
/// # Memory Layout
/// Vectors are stored contiguously as u8 for SIMD-friendly iteration.
pub struct VectorStorage {
    /// Contiguous vector data (dimension * num_vectors * 1 byte)
    data: Vec<u8>,
    /// Mapping from VectorId to data offset
    offsets: Vec<u32>,
    /// Tombstone bitmap for soft deletes
    tombstones: BitVec,
    /// Vector dimensionality
    dimensions: u32,
    /// Next available ID
    next_id: VectorId,
    /// Quantization parameters
    quantizer_config: QuantizerConfig,
}
```

### 2.4 Persistence Layer

**Purpose:** Durable storage with crash recovery.

**Inputs:**
- Write operations (insert, delete)
- Read requests (load, checkpoint)

**Outputs:**
- Acknowledged writes
- Recovered state on startup

**Invariants:**
- [INV-PERS-1] WAL entries are fsync'd before acknowledgment
- [INV-PERS-2] Snapshots are atomic (complete or absent)
- [INV-PERS-3] Recovery replays WAL after last snapshot

**Serialization Safety (v1.8 - W13.2):**

EdgeVec uses `bytemuck` for all byte-to-struct conversions in the persistence layer. This provides:

1. **Compile-time verification:** `Pod` trait ensures type is safe to cast from bytes
2. **Runtime alignment checks:** `try_cast_slice` verifies alignment before conversion
3. **No undefined behavior:** All `#[allow(clippy::cast_ptr_alignment)]` annotations removed
4. **Graceful error handling:** `PersistenceError::Corrupted` on alignment failures

Key types implementing `Pod + Zeroable`:
- `VectorId` (4 bytes, 4-byte aligned)
- `HnswNode` (24 bytes, 8-byte aligned)

```rust
/// Write-Ahead Log for crash recovery.
/// 
/// Format: [MAGIC][VERSION][ENTRY]...[ENTRY][CHECKSUM]
pub struct WriteAheadLog<B: StorageBackend> {
    backend: B,
    sequence: u64,
    buffer: Vec<u8>,
}

/// Storage backend trait for platform abstraction.
pub trait StorageBackend {
    /// Write bytes at offset (append-only for WAL)
    fn write(&mut self, data: &[u8]) -> Result<(), StorageError>;
    /// Read bytes from offset
    fn read(&self, offset: u64, len: usize) -> Result<Vec<u8>, StorageError>;
    /// Ensure durability (fsync equivalent)
    fn sync(&mut self) -> Result<(), StorageError>;
    /// Truncate to offset (for WAL compaction)
    fn truncate(&mut self, offset: u64) -> Result<(), StorageError>;
}
```

### 2.5 Quantization Pipeline (New for SQ8)

**Purpose:** Handle conversion between high-precision inputs (`f32`) and storage format (`u8`).

**Algorithm:** Min-Max Normalization (Scalar Quantization)
1. **Determine Range:** Identify global `min` and `max` values (static or learned).
2. **Normalize:** `v_norm = (v - min) / (max - min)`
3. **Scale:** `v_u8 = v_norm * 255`
4. **Rounding:** `v_u8 = round(v_u8)`

**Hybrid Mode:**
- **v1 (MVP):** `u8` Only. Original `f32` is discarded after quantization. High memory efficiency.
- **v2 (Future):** `u8` for index search, `f32` on disk for re-ranking.

**Zero Mapping:**
- If `min=0.0` and `max=1.0`, then `0.0` explicitly maps to `0`.

---

## 3. Data Flow

### 3.1 Insert Flow

```
User calls insert(vector: &[f32])
         │
         ▼
┌─────────────────────────────────────────┐
│ 1. Validate dimensions                   │
│    assert!(vector.len() == config.dim)  │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 2. Quantize Vector (f32 -> u8)           │
│    u8_vec = quantizer.quantize(vector)  │
│    // Uses global min/max config        │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 3. Allocate VectorId                     │
│    id = storage.allocate()              │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 4. Write to WAL (BEFORE in-memory)       │
│    wal.append(InsertEntry { id, vec })  │
│    wal.sync()                           │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 5. Store vector data (u8)                │
│    storage.store(id, u8_vec)            │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 6. Update HNSW index                     │
│    level = rng.sample_level()           │
│    for l in (0..=level).rev() {         │
│        neighbors = search_layer(l)      │
│        connect(id, neighbors, l)        │
│    }                                     │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 7. Return VectorId                       │
└─────────────────────────────────────────┘
```

### 3.2 Search Flow

```
User calls search(query: &[f32], k: usize)
         │
         ▼
┌─────────────────────────────────────────┐
│ 1. Validate dimensions                   │
│    assert!(query.len() == config.dim)   │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 2. Quantize Query (f32 -> u8)            │
│    q_u8 = quantizer.quantize(query)     │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 3. Get entry point (top layer)           │
│    ep = index.entry_point               │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 4. Greedy search in upper layers         │
│    for l in (top_layer..1).rev() {      │
│        ep = search_layer(q_u8, ep, 1)   │
│    }                                     │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 5. ef_search in layer 0                  │
│    candidates = search_layer(           │
│        q_u8, ep, ef_search              │
│    )                                     │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 6. Extract top-k results                 │
│    results = candidates.take(k)         │
│    Sort by distance ascending           │
└───────────────────┬─────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│ 7. Return Vec<SearchResult>              │
└─────────────────────────────────────────┘
```

### 3.3 Core Algorithms

#### 3.3.1 Algorithm 2: SEARCH-LAYER (Greedy Search)

**Source:** Implements SEARCH-LAYER from Malkov & Yashunin (2018), Section 4, Algorithm 2.

**Strategy:**
- Maintain candidates min-heap and results max-heap; stop when nearest candidate is worse than furthest result.
- **Cycle Safety:** Use `visited` bitset/hashset to prevent infinite loops.

```text
SEARCH-LAYER(q, ep, ef, lc)
Input: query q, entry point ep, candidates size ef, layer lc
Output: ef nearest neighbors to q

1. v ← ep
2. C ← {v}          // Min-heap (candidates)
3. W ← {v}          // Max-heap (results)
4. visited ← {v}
5. while |C| > 0:
6.     c ← extract_nearest(C)
7.     f ← get_furthest(W)
8.     if dist(c, q) > dist(f, q):
9.         break      // Optimization: No better candidates possible
10.    for each e in neighbors(c, lc):
11.        if e ∉ visited:
12.            visited.add(e)
13.            f ← get_furthest(W)
14.            if dist(e, q) < dist(f, q) or |W| < ef:
15.                C.add(e)
16.                W.add(e)
17.                if |W| > ef:
18.                    remove_furthest(W)
19. return W
```

---

## 4. Memory Layout Summary

| Struct | Size (bytes) | Alignment | Notes |
|:-------|:-------------|:----------|:------|
| `VectorId` | 8 | 8 | u64 wrapper |
| `NodeId` | 4 | 4 | u32, max 4B nodes |
| `HnswConfig` | 32 | 8 | Fixed at construction |
| `HnswNode` | 16 | 8 | Reduced from 24 |
| `SearchResult` | 16 | 8 | id + distance |
| `QuantizerConfig` | 8 | 4 | min/max params |

See [DATA_LAYOUT.md](DATA_LAYOUT.md) for detailed memory specifications.

---

## 5. WASM Boundary Summary

| Function | Direction | Safe? | Notes |
|:---------|:----------|:------|:------|
| `edgevec_new` | JS→Rust | ✅ | Returns handle |
| `edgevec_insert` | JS→Rust | ✅ | Takes TypedArray |
| `edgevec_search` | JS→Rust | ✅ | Returns serialized results |
| `edgevec_delete` | JS→Rust | ✅ | Takes VectorId |
| `edgevec_save` | JS→Rust | ✅ | Async, returns Promise |
| `edgevec_load` | JS→Rust | ✅ | Async, returns Promise |

See [WASM_BOUNDARY.md](WASM_BOUNDARY.md) for detailed FFI specifications.

---

## 6. Persistence Format Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│ File Format: EdgeVec Index (.evec)                                   │
├─────────────────────────────────────────────────────────────────────┤
│ Header (64 bytes)                                                    │
│ ├── Magic: "EVEC" (4 bytes)                                         │
│ ├── Version: u16 (2 bytes)                                          │
│ ├── Flags: u16 (2 bytes)                                            │
│ ├── Vector Count: u64 (8 bytes)                                     │
│ ├── Index Offset: u64 (8 bytes)                                     │
│ ├── Metadata Offset: u64 (8 bytes)                                  │
│ ├── RNG Seed: u64 (8 bytes)                                         │
│ ├── Dimensions: u32 (4 bytes)                                       │
│ ├── Header CRC: u32 (4 bytes)                                       │
│ ├── HNSW M: u32 (4 bytes)                                           │
│ ├── HNSW M0: u32 (4 bytes)                                          │
│ └── Reserved: u64 (8 bytes)                                         │
├─────────────────────────────────────────────────────────────────────┤
│ Vector Data Section                                                  │
│ └── [u8; dimensions] × vector_count                                 │
├─────────────────────────────────────────────────────────────────────┤
│ Index Section (HNSW graph)                                           │
│ ├── Layer count: u32                                                │
│ ├── Entry point: u32                                                │
│ └── Adjacency lists (Compressed VByte)                              │
├─────────────────────────────────────────────────────────────────────┤
│ Footer (16 bytes)                                                    │
│ ├── CRC32: u32                                                      │
│ └── Magic: "CEVE" (4 bytes, reverse of header)                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 7. Performance Budget

| Operation | Target | Constraint | Verification |
|:----------|:-------|:-----------|:-------------|
| Search 100k vectors | <10ms | P99 latency | Benchmark suite |
| Insert single vector | <2ms (Quant) / <5ms (F32) | Mean | Benchmark suite |
| Index load (100k) | <500ms | Cold start | Benchmark suite |
| Memory per vector (768d) | <100 bytes overhead | Total - vector data | Static analysis + test |

### Memory Budget Calculation (768 dimensions)

```
Vector data (u8):  768 * 1 byte  =   768 bytes
HNSW node:         16 bytes (fixed)
Neighbors (avg):   66 bytes (compressed)
Tombstone bit:     1/8 bytes
ID mapping:        4 bytes
─────────────────────────────────────────────
Total:             768 + 86 + 0.125 = 854.125 bytes

Overhead ratio:    86.125 / 768 = 11.2% < 100 bytes ✅
```

---

## 8. Open Questions

- [Q1] Should we support int8 quantization in v1? **[DECIDED]** — Yes, SQ8 (u8) is the primary storage for v1.
- [Q2] How to handle SharedArrayBuffer not available? **[FACT]** — Fallback to single-threaded
- [Q3] IndexedDB transaction size limits? **[RISK_ACCEPTED]** — Writes > 50MB must be chunked by the application layer (for v1).

---

## 9. Approval Status

| Reviewer | Verdict | Date |
|:---------|:--------|:-----|
| HOSTILE_REVIEWER | [PENDING] | |

---

## 10. Verification Strategy Reference

See [TEST_STRATEGY.md](TEST_STRATEGY.md) for the complete verification plan including:
- Property-based tests (proptest)
- Fuzzing campaigns (cargo-fuzz)
- Memory safety verification (miri)
- Deterministic simulation

**"Nvidia Grade" Rule:** Every component above has corresponding verification defined in TEST_STRATEGY.md.

---

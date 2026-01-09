# RFC: Sparse Vectors for EdgeVec

**RFC ID:** RFC-007
**Author:** META_ARCHITECT
**Date:** 2026-01-08
**Status:** PROPOSED
**Target Version:** v0.9.0

---

## Summary

This RFC defines the design and implementation plan for sparse vector support in EdgeVec, enabling hybrid search that combines dense semantic embeddings with sparse keyword features (BM25, TF-IDF).

---

## Motivation

### Problem Statement

EdgeVec currently only supports dense vectors (fixed-dimension Float32Array). Users cannot:

1. **Combine semantic and keyword search** — No hybrid search capability
2. **Store BM25 vectors** — Common for text retrieval systems
3. **Efficiently handle high-dimensional sparse data** — Feature vectors with 10k+ dimensions but <100 non-zero values

### User Demand

From RFC-006 (BM25 Hybrid Search):

> "is BM25 support on roadmap here? I would love to build pretty simple hybrid search" — Reddit user Lucas

From ROADMAP v6.1:
> v0.9.0: Sparse Vectors + Hybrid Search + Flat Index

### Industry Context

| Database | Sparse Support | Release |
|:---------|:---------------|:--------|
| Milvus 2.5 | Sparse-BM25, 30x faster | Dec 2024 |
| Qdrant | Sparse vectors | 2024 |
| Weaviate | BM25/BM25F + RRF | 2024 |
| Pinecone | Sparse vectors | 2024 |
| **EdgeVec** | **Proposed** | **v0.9.0** |

---

## Use Cases

### UC1: Hybrid Search (Primary)

Combine dense semantic embeddings with sparse BM25 vectors:

```javascript
// User computes BM25 sparse vector externally
const sparseQuery = new SparseVector([
  [1024, 0.85],  // term "database" -> weight 0.85
  [3721, 0.42],  // term "vector" -> weight 0.42
  [8190, 0.91],  // term "search" -> weight 0.91
]);

// Hybrid search: dense + sparse
const results = db.hybridSearch({
  dense: denseEmbedding,
  sparse: sparseQuery,
  alpha: 0.7,  // 70% dense, 30% sparse
  k: 10,
});
```

### UC2: Multi-Feature Matching

High-dimensional categorical features with sparse representation:

```javascript
// User profile: 50k possible features, only 23 active
const userFeatures = new SparseVector([
  [142, 1.0],    // likes_jazz
  [891, 1.0],    // age_25_34
  [2341, 0.8],   // interest_tech
  // ... 20 more features
]);

const similarUsers = db.sparseSearch(userFeatures, 10);
```

### UC3: TF-IDF Document Search

Classic information retrieval with term frequency vectors:

```javascript
// Document represented as TF-IDF sparse vector
const docVector = new SparseVector([
  [vocab["machine"], 0.23],
  [vocab["learning"], 0.31],
  [vocab["neural"], 0.19],
]);

db.insertSparse(docVector);
```

---

## Design

### Data Structure: CSR Format

Following industry standard (sprs CsVec, Milvus), use **Compressed Sparse Row (CSR)** format:

```rust
/// Sparse vector using CSR format
/// Memory: 4 bytes (index) + 4 bytes (value) per non-zero = 8 bytes/element
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SparseVector {
    /// Sorted indices of non-zero elements (ascending)
    indices: Vec<u32>,
    /// Values corresponding to each index
    values: Vec<f32>,
    /// Maximum dimension (vocabulary size, feature space)
    dim: u32,
}
```

**Memory Layout:**
```
┌─────────────────────────────────────────────────────────┐
│                    SparseVector                         │
├─────────────────────────────────────────────────────────┤
│  indices: [u32; N]   │  values: [f32; N]   │  dim: u32  │
│  [1024, 3721, 8190]  │  [0.85, 0.42, 0.91] │  10000     │
└─────────────────────────────────────────────────────────┘
```

**Size Comparison:**
| Representation | 10k-dim, 50 non-zero | 10k-dim, 100 non-zero |
|:---------------|:---------------------|:----------------------|
| Dense Float32 | 40,000 bytes | 40,000 bytes |
| **Sparse CSR** | **400 bytes** | **800 bytes** |
| Compression | **100x** | **50x** |

### Distance Metrics

#### Dot Product (Primary)

For BM25 and TF-IDF, dot product is the standard metric:

```rust
/// Sparse dot product - O(min(|a|, |b|)) via merge-intersection
pub fn sparse_dot_product(a: &SparseVector, b: &SparseVector) -> f32 {
    let mut result = 0.0f32;
    let mut i = 0;
    let mut j = 0;

    // Merge-intersection algorithm
    while i < a.indices.len() && j < b.indices.len() {
        match a.indices[i].cmp(&b.indices[j]) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                result += a.values[i] * b.values[j];
                i += 1;
                j += 1;
            }
        }
    }
    result
}
```

**Complexity:** O(|a| + |b|) worst case, O(min(|a|, |b|)) best case when indices perfectly overlap. The merge-intersection algorithm traverses both arrays, advancing pointers based on index comparison.

#### Cosine Similarity (Secondary)

```rust
/// Sparse cosine = dot(a, b) / (||a|| * ||b||)
pub fn sparse_cosine(a: &SparseVector, b: &SparseVector) -> f32 {
    let dot = sparse_dot_product(a, b);
    let norm_a = a.norm();  // Pre-computed or cached
    let norm_b = b.norm();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
```

### Storage Strategy

Extend existing `VectorStorage` with sparse support:

```rust
/// Extended storage supporting both dense and sparse vectors
pub struct HybridStorage {
    /// Dense vector storage (existing)
    dense: VectorStorage,

    /// Sparse vector storage (new)
    sparse: SparseStorage,

    /// Mapping from VectorId to storage type
    vector_types: HashMap<VectorId, VectorType>,
}

/// Unified ID for hybrid storage. High bit indicates type:
/// - 0x0... = Dense (maps to VectorId)
/// - 0x8... = Sparse (maps to SparseId)
/// - HybridId stores both for paired vectors
pub enum VectorType {
    Dense(VectorId),
    Sparse(SparseId),
    /// Paired dense+sparse: same user-facing ID, separate internal storage
    Hybrid { user_id: u64, dense_id: VectorId, sparse_id: SparseId },
}

pub struct SparseStorage {
    /// Packed indices: all vectors' indices concatenated
    indices: Vec<u32>,
    /// Packed values: all vectors' values concatenated
    values: Vec<f32>,
    /// Offsets into packed arrays: offset[i] = start of vector i
    offsets: Vec<u32>,
    /// Maximum dimension for each stored vector
    dims: Vec<u32>,
    /// Deletion bitmap
    deleted: BitVec,
    /// Next ID
    next_id: u64,
}
```

**Memory Estimate (100k sparse vectors, avg 50 non-zero):**
```
indices: 100k * 50 * 4 bytes = 20 MB
values:  100k * 50 * 4 bytes = 20 MB
offsets: 100k * 4 bytes      = 0.4 MB
dims:    100k * 4 bytes      = 0.4 MB
deleted: 100k / 8 bytes      = 12.5 KB
─────────────────────────────────────
Total: ~41 MB (vs 4 GB for dense 10k-dim)
```

**Memory Estimate (1M sparse vectors, avg 50 non-zero):**
```
indices: 1M * 50 * 4 bytes = 200 MB
values:  1M * 50 * 4 bytes = 200 MB
offsets: 1M * 4 bytes      = 4 MB
dims:    1M * 4 bytes      = 4 MB
deleted: 1M / 8 bytes      = 125 KB
─────────────────────────────────────
Sparse subtotal: ~408 MB

Inverted index (optional):
  - Vocabulary: 100k unique terms
  - Avg docs per term: 500
  - Posting: 100k * 500 * 8 bytes = 400 MB
  - With cap at 10k docs/term: ~80 MB
─────────────────────────────────────
Total with inverted index: ~488-808 MB
Total sparse-only: ~408 MB

[FACT] Fits within Safari ~1GB WASM limit with headroom
```

### Inverted Index (Optional, Phase 2)

For efficient sparse search, build inverted index:

```rust
/// Inverted index: dimension -> vectors containing that dimension
pub struct SparseInvertedIndex {
    /// posting_lists[dim] = [(vector_id, value), ...]
    /// NOTE: Capped at MAX_POSTING_LENGTH (10,000) to bound memory.
    /// For high-frequency terms, only top-scoring vectors are kept.
    posting_lists: HashMap<u32, Vec<(SparseId, f32)>>,
}

const MAX_POSTING_LENGTH: usize = 10_000;
```

**Search Algorithm:**
1. For query sparse vector Q with indices [i1, i2, ...]
2. Retrieve posting lists for each index
3. Accumulate scores per candidate vector
4. Return top-k by score

**Complexity:** O(Q_nnz * avg_posting_length + k log k)

### WASM Bindings

```typescript
// TypeScript API
export class SparseVector {
  constructor(entries: [number, number][] | { indices: Uint32Array, values: Float32Array });

  readonly indices: Uint32Array;
  readonly values: Float32Array;
  readonly nnz: number;  // Number of non-zeros
  readonly dim: number;

  dot(other: SparseVector): number;
  cosine(other: SparseVector): number;
  norm(): number;
  normalize(): SparseVector;

  toJSON(): { indices: number[], values: number[], dim: number };
  static fromJSON(json: object): SparseVector;
}

// EdgeVec API extensions
export class EdgeVec {
  // Existing dense methods...

  // New sparse methods
  insertSparse(vector: SparseVector): number;
  insertSparseBatch(vectors: SparseVector[]): InsertResult;

  sparseSearch(query: SparseVector, k: number): SearchResult[];

  // Hybrid insert (dense + sparse pair)
  insertHybrid(dense: Float32Array, sparse: SparseVector): number;
  insertHybridBatch(pairs: Array<{dense: Float32Array, sparse: SparseVector}>): InsertResult;

  // Hybrid search (dense + sparse)
  hybridSearch(options: {
    dense?: Float32Array;
    sparse?: SparseVector;
    alpha?: number;  // Weight for dense (0-1)
    k: number;
    filter?: string;
  }): SearchResult[];
}
```

### Hybrid Search Algorithm

Reciprocal Rank Fusion (RRF) for combining dense and sparse results:

```rust
/// RRF: combined_score = sum(1 / (k + rank_i))
pub fn reciprocal_rank_fusion(
    dense_results: &[SearchResult],
    sparse_results: &[SearchResult],
    k_constant: f32,  // Typically 60
) -> Vec<SearchResult> {
    let mut scores: HashMap<VectorId, f32> = HashMap::new();

    // Score from dense results
    for (rank, result) in dense_results.iter().enumerate() {
        *scores.entry(result.id).or_default() += 1.0 / (k_constant + rank as f32 + 1.0);
    }

    // Score from sparse results
    for (rank, result) in sparse_results.iter().enumerate() {
        *scores.entry(result.id).or_default() += 1.0 / (k_constant + rank as f32 + 1.0);
    }

    // Sort by combined score
    let mut combined: Vec<_> = scores.into_iter().collect();
    combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    combined.into_iter()
        .map(|(id, score)| SearchResult { id, score })
        .collect()
}
```

**Alternative: Weighted Sum**
```rust
/// Weighted sum: score = alpha * norm(dense_score) + (1 - alpha) * norm(sparse_score)
pub fn weighted_sum_fusion(
    dense_results: &[SearchResult],
    sparse_results: &[SearchResult],
    alpha: f32,  // 0.0 = sparse only, 1.0 = dense only
) -> Vec<SearchResult> {
    // Step 1: Min-max normalization to [0, 1]
    let dense_norm = min_max_normalize(dense_results);
    let sparse_norm = min_max_normalize(sparse_results);

    // Step 2: Build score map
    let mut scores: HashMap<VectorId, f32> = HashMap::new();

    for (id, score) in dense_norm {
        *scores.entry(id).or_default() += alpha * score;
    }
    for (id, score) in sparse_norm {
        *scores.entry(id).or_default() += (1.0 - alpha) * score;
    }

    // Step 3: Sort and return
    let mut combined: Vec<_> = scores.into_iter().collect();
    combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    combined.into_iter()
        .map(|(id, score)| SearchResult { id, score })
        .collect()
}

/// Min-max normalization: (x - min) / (max - min) → [0, 1]
fn min_max_normalize(results: &[SearchResult]) -> Vec<(VectorId, f32)> {
    if results.is_empty() { return vec![]; }
    let min = results.iter().map(|r| r.score).fold(f32::INFINITY, f32::min);
    let max = results.iter().map(|r| r.score).fold(f32::NEG_INFINITY, f32::max);
    let range = max - min;
    if range == 0.0 {
        return results.iter().map(|r| (r.id, 1.0)).collect();
    }
    results.iter().map(|r| (r.id, (r.score - min) / range)).collect()
}
```

---

## API Design

### Rust API

```rust
// Core types
pub use sparse::{SparseVector, SparseId, SparseStorage};
pub use sparse::metrics::{sparse_dot_product, sparse_cosine};

// Construction
impl SparseVector {
    /// Create from sorted indices and values. Validates: sorted, no duplicates, no NaN, nnz >= 1.
    pub fn new(indices: Vec<u32>, values: Vec<f32>, dim: u32) -> Result<Self, SparseError>;
    /// Create from unsorted pairs. Sorts internally. Validates: no duplicates, no NaN, nnz >= 1.
    pub fn from_pairs(pairs: &[(u32, f32)], dim: u32) -> Result<Self, SparseError>;
    /// Create with single element (minimum valid sparse vector).
    pub fn singleton(index: u32, value: f32, dim: u32) -> Result<Self, SparseError>;
}

// Operations
impl SparseVector {
    pub fn dot(&self, other: &SparseVector) -> f32;
    pub fn cosine(&self, other: &SparseVector) -> f32;
    pub fn norm(&self) -> f32;
    pub fn nnz(&self) -> usize;
    pub fn dim(&self) -> u32;
}

// Storage operations
impl SparseStorage {
    pub fn insert(&mut self, vector: &SparseVector) -> Result<SparseId, StorageError>;
    pub fn get(&self, id: SparseId) -> Option<SparseVector>;
    pub fn delete(&mut self, id: SparseId) -> Result<bool, StorageError>;
}

// Search operations
impl SparseSearcher {
    /// Returns SearchResult with unified ID (same ID space as dense vectors via HybridStorage)
    pub fn search(&self, query: &SparseVector, k: usize) -> Vec<SearchResult>;
}

// Hybrid search
impl HybridSearcher {
    pub fn search(&self, config: HybridSearchConfig) -> Vec<SearchResult>;
}

pub struct HybridSearchConfig {
    pub dense_query: Option<&[f32]>,
    pub sparse_query: Option<&SparseVector>,
    pub alpha: f32,           // Dense weight (0-1)
    pub k: usize,
    pub fusion: FusionMethod,
}

pub enum FusionMethod {
    Rrf { k_constant: f32 },
    WeightedSum,
}
```

### TypeScript API

```typescript
import init, { EdgeVec, SparseVector } from 'edgevec';

await init();

// Create sparse vector from pairs
const sparse = new SparseVector([
  [1024, 0.85],
  [3721, 0.42],
  [8190, 0.91],
], 10000);  // dim = 10000

// Or from typed arrays
const sparse2 = new SparseVector({
  indices: new Uint32Array([1024, 3721, 8190]),
  values: new Float32Array([0.85, 0.42, 0.91]),
  dim: 10000,
});

// Operations
console.log(sparse.nnz);          // 3
console.log(sparse.dim);          // 10000
console.log(sparse.dot(sparse2)); // dot product
console.log(sparse.norm());       // L2 norm

// Insert into database
const db = new EdgeVec(config);
const id = db.insertSparse(sparse);

// Sparse-only search
const results = db.sparseSearch(sparse, 10);

// Hybrid search (dense + sparse)
const hybridResults = db.hybridSearch({
  dense: denseEmbedding,
  sparse: sparse,
  alpha: 0.7,  // 70% dense, 30% sparse
  k: 10,
});
```

---

## Performance Targets

| Operation | P50 Target | P99 Target | Constraint |
|:----------|:-----------|:-----------|:-----------|
| Dot product (50 nnz) | <300ns | <500ns | Single-threaded |
| Dot product (100 nnz) | <600ns | <1μs | Single-threaded |
| Insert sparse vector | <50μs | <100μs | Including validation |
| Sparse search (100k, k=10) | <3ms | <5ms | Brute force |
| Sparse search (100k, k=10) | <500μs | <1ms | With inverted index |
| Hybrid search (100k, k=10) | <6ms | <10ms | Dense HNSW + Sparse brute |

### Benchmarks to Create

```rust
#[bench]
fn bench_sparse_dot_50_nnz(b: &mut Bencher) {
    let a = random_sparse(10000, 50);
    let b = random_sparse(10000, 50);
    b.iter(|| sparse_dot_product(&a, &b));
}

#[bench]
fn bench_sparse_search_100k(b: &mut Bencher) {
    let storage = create_sparse_storage(100_000, 50);
    let query = random_sparse(10000, 50);
    b.iter(|| storage.brute_force_search(&query, 10));
}
```

---

## Memory Budget

| Component | 100k vectors | Estimate |
|:----------|:-------------|:---------|
| Sparse indices (50 nnz avg) | 100k * 50 * 4 | 20 MB |
| Sparse values (50 nnz avg) | 100k * 50 * 4 | 20 MB |
| Offsets array | 100k * 4 | 0.4 MB |
| Dimension array | 100k * 4 | 0.4 MB |
| Deletion bitmap | 100k / 8 | 12.5 KB |
| **Sparse subtotal** | | **~41 MB** |
| Inverted index (optional) | ~50 MB | Depends on vocabulary |
| **Total sparse** | | **~41-91 MB** |

**Bundle Size Impact:**
- Core sparse types: +15 KB
- Sparse metrics: +5 KB
- Sparse storage: +20 KB
- WASM bindings: +10 KB
- **Total: <50 KB added** (10% of current 477 KB)

---

## Implementation Plan

### Phase 1: Core Types (Week 37-38)

**Files to Create:**
```
src/sparse/
├── mod.rs           # Module exports
├── vector.rs        # SparseVector struct
├── metrics.rs       # dot_product, cosine
├── storage.rs       # SparseStorage
└── error.rs         # SparseError enum
```

**Tasks:**
1. Define `SparseVector` with CSR format
2. Implement validation (sorted indices, no duplicates, no NaN)
3. Implement `sparse_dot_product` and `sparse_cosine`
4. Property tests for metric correctness

### Phase 2: Storage & Persistence (Week 38-39)

**Tasks:**
1. Implement `SparseStorage` with packed arrays
2. Add serialization (same pattern as dense storage)
3. Add deletion support (BitVec)
4. WAL integration (entry type 2 = Sparse Insert)

### Phase 3: Search (Week 39-40)

**Tasks:**
1. Implement brute-force sparse search
2. Implement inverted index (optional optimization)
3. Add `sparseSearch` to EdgeVec API

### Phase 4: Hybrid Search (Week 40-41)

**Tasks:**
1. Implement RRF fusion
2. Implement weighted sum fusion
3. Add `hybridSearch` to EdgeVec API
4. Benchmark hybrid vs dense-only vs sparse-only

### Phase 5: WASM Bindings (Week 41-42)

**Tasks:**
1. Export `SparseVector` to WASM
2. Add TypeScript type definitions
3. Implement `insertSparse`, `sparseSearch`, `hybridSearch`
4. Update npm package

---

## Acceptance Criteria

### Functional Requirements

- [ ] `SparseVector` stores sorted indices and values
- [ ] Validation rejects: unsorted indices, duplicate indices, NaN values, empty vectors
- [ ] `sparse_dot_product` returns correct value (verified vs dense computation)
- [ ] `sparse_cosine` returns value in [-1, 1] range
- [ ] `SparseStorage` supports insert, get, delete operations
- [ ] Sparse vectors persist across save/load cycles
- [ ] `sparseSearch` returns k nearest neighbors by dot product
- [ ] `hybridSearch` combines dense and sparse results correctly

### Performance Requirements

- [ ] Dot product (50 nnz) < 500ns
- [ ] Dot product (100 nnz) < 1μs
- [ ] Sparse search (100k vectors, k=10) < 5ms
- [ ] Bundle size increase < 100 KB

### Test Coverage Requirements

- [ ] Unit tests for SparseVector construction and validation
- [ ] Unit tests for all metric functions
- [ ] Property tests: dot(a, b) == dot(b, a) (commutativity)
- [ ] Property tests: dot(a, a) >= 0 (positive semi-definite)
- [ ] Property tests: cosine(a, a) == 1.0 for non-zero vectors
- [ ] Integration tests for sparse storage roundtrip
- [ ] Integration tests for hybrid search
- [ ] WASM tests in browser environment

### Documentation Requirements

- [ ] API documentation for all public types and functions
- [ ] Usage examples in docs/guides/SPARSE_VECTORS.md
- [ ] Hybrid search tutorial
- [ ] Performance tuning guide for sparse vectors

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| Performance below targets | LOW | HIGH | Profile early, SIMD for hot paths |
| Memory overhead too high | LOW | MEDIUM | Packed storage format, optional inverted index |
| WASM binding complexity | MEDIUM | MEDIUM | Follow existing dense vector patterns |
| Inverted index complexity | MEDIUM | LOW | Make inverted index optional (Phase 2) |

---

## Alternatives Considered

### A1: External Sparse Library (sprs)

**Pros:**
- Battle-tested implementation
- Rich feature set

**Cons:**
- Adds ~500 KB to bundle
- WASM compatibility unknown
- Over-engineered for our needs

**Decision:** Build minimal custom implementation optimized for our use case.

### A2: Dense-Only Hybrid (User Computes Externally)

**Pros:**
- No code changes
- Documented in RFC-006

**Cons:**
- Poor user experience
- Competitive disadvantage
- Missing market opportunity

**Decision:** Native sparse support aligns with v0.9.0 roadmap goals.

### A3: Full BM25 Integration

**Pros:**
- Complete solution
- No external dependencies for user

**Cons:**
- +150 KB bundle size (tokenizer)
- Scope creep for v0.9.0
- Text storage not in EdgeVec's focus

**Decision:** Defer full BM25 to v0.10.0+. Sparse vectors enable user-computed BM25.

---

## References

- [sprs CsVec Documentation](https://docs.rs/sprs/latest/sprs/struct.CsVecBase.html)
- [Milvus 2.5 Sparse-BM25](https://milvus.io/docs/sparse_vector.md)
- [RFC-006: BM25 Hybrid Search](./RFC_BM25_HYBRID_SEARCH.md)
- [EdgeVec ROADMAP v6.1](../planning/ROADMAP.md)

---

## Appendix A: Codebase Patterns to Reuse

Based on exploration of EdgeVec codebase:

| Pattern | Source | Application |
|:--------|:-------|:------------|
| `Metric<T>` trait | `src/metric/mod.rs` | `SparseMetic` trait |
| SIMD dispatch | `src/metric/l2.rs` | Sparse dot product optimization |
| `VectorStorage` layout | `src/storage/mod.rs` | `SparseStorage` design |
| BitVec deletion | `src/storage/mod.rs` | Sparse deletion tracking |
| WAL integration | `src/storage/mod.rs` | Sparse WAL entries |
| `SearchContext` | `src/hnsw/search.rs` | Sparse search context |
| Thiserror enums | `src/error.rs` | `SparseError` |
| Serde tagged enums | `src/metadata/types.rs` | Sparse serialization |

---

## Appendix B: Example Usage

### Hybrid Semantic + Keyword Search

```javascript
import init, { EdgeVec, EdgeVecConfig, SparseVector } from 'edgevec';

await init();

// Configure for hybrid search
const config = new EdgeVecConfig(384);  // Dense dimension
config.metric = 'cosine';

const db = new EdgeVec(config);

// Insert documents with both dense and sparse representations
for (const doc of documents) {
  // Dense: semantic embedding from sentence-transformers
  const denseEmbed = await embedder.embed(doc.text);

  // Sparse: BM25 vector from external tokenizer
  const sparseEmbed = bm25.encode(doc.text);

  // Insert hybrid
  db.insertHybrid(denseEmbed, new SparseVector(sparseEmbed));
}

await db.save('hybrid-index');

// Search with hybrid query
const query = "machine learning tutorial";
const denseQuery = await embedder.embed(query);
const sparseQuery = new SparseVector(bm25.encode(query));

const results = db.hybridSearch({
  dense: denseQuery,
  sparse: sparseQuery,
  alpha: 0.6,  // 60% semantic, 40% keyword
  k: 10,
});

console.log(results);
// [{ id: 42, score: 0.89 }, { id: 17, score: 0.85 }, ...]
```

---

**Status:** [APPROVED] — Passed HOSTILE_REVIEWER validation 2026-01-08

**Approval Notes:**
- All 3 critical issues addressed (complexity, 1M scale, insertHybrid API)
- All 4 major issues addressed (P50/P99, empty constructor, ID namespaces, WeightedSum)
- All 3 minor issues addressed (posting cap, timeline, SearchResult ID)
- Review document: `docs/reviews/2026-01-08_RFC_SPARSE_VECTORS_APPROVED.md`

**Next Steps:**
1. Begin Phase 1 implementation (Week 37)
2. Create `src/sparse/` module structure

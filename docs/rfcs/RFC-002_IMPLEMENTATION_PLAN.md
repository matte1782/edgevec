# RFC-002: Implementation Plan

**Document:** W25.6.4 — Implementation Scope Definition
**Author:** PLANNER
**Date:** 2025-12-20
**Status:** [APPROVED]
**RFC Reference:** RFC-002 (APPROVED)
**Scale-Up Reference:** SCALE_UP_ANALYSIS_2025-12-20.md

---

## 1. Executive Summary

This document defines the v0.6.0 implementation scope, combining:
1. **RFC-002: Integrated Metadata Storage** (APPROVED)
2. **Binary Quantization** (APPROVED from Scale-Up Analysis)

**Total Estimated Effort:** 140 hours (3.5 weeks) + 30% contingency = ~182 hours (~4.5 weeks)
**Target Release:** v0.6.0

---

## 2. v0.6.0 Feature Overview

### 2.1 Metadata Storage (RFC-002)

| Capability | Description |
|:-----------|:------------|
| `insertWithMetadata()` | Atomic vector + metadata insert |
| `searchFiltered()` | WASM-side filter evaluation |
| `getMetadata()` | Retrieve metadata by ID |
| Automatic cleanup | Metadata deleted with soft_delete |
| Unified persistence | v0.4 format with metadata section |

### 2.2 Binary Quantization (Scale-Up Analysis)

| Capability | Description |
|:-----------|:------------|
| Sign-based BQ | 32x memory compression |
| SIMD popcount | Hardware-accelerated Hamming distance |
| Top-K rescoring | Recall recovery with F32 rerank |
| Hybrid mode | BQ for candidates, F32 for final ranking |

---

## 3. Implementation Components

### 3.1 Metadata Storage Components

| Component | Files | Hours | Risk | Dependencies |
|:----------|:------|:------|:-----|:-------------|
| HnswIndex integration | `src/hnsw/graph.rs` | 4 | Low | None |
| `insert_with_metadata()` | `src/hnsw/operations.rs` | 4 | Low | MetadataStore |
| `soft_delete()` cleanup | `src/hnsw/operations.rs` | 2 | Low | MetadataStore |
| `compact()` metadata | `src/hnsw/operations.rs` | 2 | Low | MetadataStore |
| `search_filtered()` | `src/hnsw/search.rs` | 6 | Medium | Filter module |
| Selectivity estimation | `src/filter/selectivity.rs` | 4 | Medium | None |
| MetadataSectionHeader | `src/persistence/header.rs` | 2 | Low | None |
| Postcard serialization | `src/metadata/serialize.rs` | 4 | Low | postcard crate |
| v0.4 write_snapshot | `src/persistence/snapshot.rs` | 4 | Medium | Header |
| v0.4 read_snapshot | `src/persistence/snapshot.rs` | 4 | Medium | Header |
| v0.3→v0.4 migration | `src/persistence/migration.rs` | 3 | Low | read_snapshot |
| WASM bindings | `src/wasm/metadata.rs` | 6 | Medium | All Rust |
| TypeScript types | `pkg/edgevec.d.ts` | 2 | Low | WASM |
| Unit tests | `tests/metadata_*.rs` | 6 | Low | Implementation |
| Integration tests | `tests/integration/metadata.rs` | 4 | Low | WASM |

**Subtotal: 57 hours**

### 3.2 Binary Quantization Components

| Component | Files | Hours | Risk | Dependencies |
|:----------|:------|:------|:-----|:-------------|
| BinaryVector type | `src/quantization/binary.rs` | 4 | Low | None |
| Sign-based encoding | `src/quantization/binary.rs` | 4 | Low | BinaryVector |
| SIMD popcount (x86) | `src/simd/popcount_x86.rs` | 6 | Medium | None |
| SIMD popcount (ARM) | `src/simd/popcount_arm.rs` | 6 | Medium | None |
| Hamming distance | `src/quantization/hamming.rs` | 4 | Low | popcount |
| BinaryVectorStorage | `src/storage/binary.rs` | 6 | Medium | BinaryVector |
| BQ search | `src/hnsw/search_bq.rs` | 8 | High | BinaryVectorStorage |
| Rescoring layer | `src/hnsw/rescore.rs` | 6 | Medium | F32 storage |
| Hybrid search | `src/hnsw/hybrid.rs` | 8 | High | BQ + rescore |
| BQ persistence | `src/persistence/binary.rs` | 4 | Low | Postcard |
| WASM bindings | `src/wasm/binary.rs` | 4 | Medium | All Rust |
| Unit tests | `tests/binary_*.rs` | 8 | Low | Implementation |
| Benchmarks | `benches/binary_*.rs` | 6 | Low | Implementation |

**Subtotal: 74 hours**

### 3.3 Shared/Infrastructure

| Component | Files | Hours | Risk | Dependencies |
|:----------|:------|:------|:-----|:-------------|
| Memory pressure API | `src/wasm/memory.rs` | 4 | Low | None |
| CHANGELOG update | `CHANGELOG.md` | 1 | None | None |
| README update | `README.md` | 2 | None | None |
| Migration guide | `docs/MIGRATION_v0.5_to_v0.6.md` | 2 | None | None |

**Subtotal: 9 hours**

---

## 4. Implementation Phases

### Phase 1: Core Metadata (Week 1, 32 hours)

**Objective:** Metadata storage integrated with HNSW, no WASM yet.

| Day | Tasks | Hours |
|:----|:------|:------|
| 1 | HnswIndex + insert_with_metadata() | 8 |
| 2 | soft_delete cleanup + compact metadata | 4 |
| 2 | search_filtered() basic | 4 |
| 3 | Selectivity estimation | 4 |
| 3 | Unit tests | 4 |
| 4 | MetadataSectionHeader + Postcard | 6 |
| 5 | v0.4 persistence read/write | 8 |

**Exit Criteria:**
- `cargo test` passes
- v0.3 → v0.4 migration works
- search_filtered() returns correct results

### Phase 2: Binary Quantization (Week 2, 48 hours)

**Objective:** BQ storage and search working with SIMD.

| Day | Tasks | Hours |
|:----|:------|:------|
| 1 | BinaryVector + sign encoding | 8 |
| 2 | SIMD popcount (x86 + ARM) | 12 |
| 3 | Hamming distance + BinaryVectorStorage | 10 |
| 4 | BQ search + rescoring | 14 |
| 5 | Benchmarks + unit tests | 8 |

**Exit Criteria:**
- `cargo test` passes with BQ enabled
- BQ search achieves >0.90 recall with rescoring
- 3x+ speedup vs F32 search

### Phase 3: WASM & Integration (Week 3, 40 hours)

**Objective:** Full WASM API, integration tests, documentation.

| Day | Tasks | Hours |
|:----|:------|:------|
| 1 | Metadata WASM bindings | 6 |
| 1 | BQ WASM bindings | 4 |
| 2 | Memory pressure monitoring | 4 |
| 2 | TypeScript types | 2 |
| 3 | Integration tests | 8 |
| 4 | Hybrid search (BQ + metadata filter) | 8 |
| 5 | Documentation + CHANGELOG | 4 |
| 5 | Release prep | 4 |

**Exit Criteria:**
- `wasm-pack test` passes
- Browser demo works
- v0.6.0 ready for release

---

## 5. Risk Analysis

### 5.1 High Risk Items

| Risk | Probability | Impact | Mitigation |
|:-----|:------------|:-------|:-----------|
| BQ recall degradation | Medium | High | Rescoring layer, benchmark validation |
| Hybrid search complexity | Medium | High | Start with simple post-filter |
| SIMD ARM NEON support | Low | Medium | Runtime detection, scalar fallback |

### 5.2 Medium Risk Items

| Risk | Probability | Impact | Mitigation |
|:-----|:------------|:-------|:-----------|
| v0.4 migration edge cases | Low | Medium | Extensive migration tests |
| Selectivity estimation accuracy | Medium | Low | Conservative default (50%) |
| WASM memory pressure | Low | Medium | Graceful degradation |

### 5.3 Dependencies

```
BinaryVector → SIMD popcount → Hamming → BinaryVectorStorage → BQ Search → Hybrid
                                                                          ↓
MetadataStore → HnswIndex → search_filtered() ─────────────────────────────→
                         ↓
                    Persistence v0.4 → WASM bindings
```

---

## 6. Testing Strategy

### 6.1 Unit Tests

| Module | Test Cases |
|:-------|:-----------|
| BinaryVector | Encode/decode, sign preservation, dimension handling |
| Hamming | Distance calculation, SIMD vs scalar equivalence |
| MetadataStore | Insert/get/delete, limits, serialization |
| search_filtered | Filter parsing, selectivity, result ordering |

### 6.2 Property Tests

| Invariant | Generator |
|:----------|:----------|
| BQ distance is symmetric | Random vector pairs |
| Metadata survives round-trip | Arbitrary metadata values |
| Filter selectivity is [0,1] | Random filter expressions |
| Rescoring preserves top-k | Random query, varying k |

### 6.3 Integration Tests

| Test | Description |
|:-----|:------------|
| metadata_roundtrip | Insert with metadata, save, load, search |
| bq_recall_benchmark | Measure recall at various k values |
| hybrid_search | BQ + metadata filter combined |
| migration_v03_v04 | Load v0.3 file, verify metadata empty |

### 6.4 Benchmarks

| Benchmark | Target |
|:----------|:-------|
| BQ search latency | <5ms for 100K vectors |
| BQ memory usage | <100 bytes/vector (768D) |
| Filter evaluation | <1μs per vector |
| Hybrid search | <10ms for 100K + filter |

---

## 7. API Summary

### 7.1 Rust API

```rust
// Metadata
pub fn insert_with_metadata(&mut self, vector: &[f32], metadata: HashMap<String, MetadataValue>) -> Result<VectorId, GraphError>;
pub fn search_filtered(&self, query: &[f32], filter: &str, k: usize) -> Result<Vec<(VectorId, f32)>, GraphError>;
pub fn get_metadata(&self, id: VectorId) -> Option<&HashMap<String, MetadataValue>>;

// Binary Quantization
pub fn insert_bq(&mut self, vector: &[f32]) -> Result<VectorId, GraphError>;
pub fn search_bq(&self, query: &[f32], k: usize) -> Result<Vec<(VectorId, f32)>, GraphError>;
pub fn search_hybrid(&self, query: &[f32], filter: Option<&str>, k: usize) -> Result<Vec<(VectorId, f32)>, GraphError>;
```

### 7.2 WASM/JavaScript API

```typescript
// Metadata
insertWithMetadata(vector: Float32Array, metadata: Record<string, MetadataValue>): number;
searchFiltered(query: Float32Array, options: FilteredSearchOptions): SearchResult[];
getMetadata(id: number): Record<string, MetadataValue> | null;

// Binary Quantization
insertBQ(vector: Float32Array): number;
searchBQ(query: Float32Array, k: number): SearchResult[];
searchHybrid(query: Float32Array, options: HybridSearchOptions): SearchResult[];

// Memory Monitoring
getMemoryPressure(): MemoryPressure;
```

---

## 8. Success Metrics

| Metric | Target |
|:-------|:-------|
| BQ memory reduction | 32x vs F32 |
| BQ search speedup | 3-5x vs F32 (see rationale below) |
| BQ recall (with rescore) | >0.90 @ k=10 |
| Filter evaluation | <1μs per vector |
| Metadata overhead | <50 bytes (empty) |
| WASM boundary crossings | 1 per search |
| v0.3 → v0.4 migration | 100% success |

### 8.1 BQ Speedup Target Rationale

The 3-5x speedup target is **intentionally conservative** compared to industry claims (8-40x):

**Why Conservative:**
1. **First Implementation:** This is EdgeVec's first BQ implementation; conservative targets reduce risk
2. **WASM Overhead:** WASM adds ~20-30% overhead vs native; industry benchmarks are native
3. **Rescoring Cost:** Our design uses F32 rescoring for recall recovery, which reduces net speedup
4. **Embedding Variability:** BQ performance varies by embedding model; not all achieve 40x
5. **Honest Claims:** We prefer to under-promise and over-deliver

**Industry Context:**
- Qdrant claims up to 40x speedup (native C++, no rescoring)
- Elastic claims 8-30x (native, with AVX2/NEON SIMD)
- Weaviate claims memory reduction, less speedup emphasis

**Revision Path:**
If benchmarks show >5x speedup, we will revise targets upward in v0.6.1.
If benchmarks show <3x speedup, we will investigate SIMD optimization gaps.

---

## 9. Schedule

| Week | Phase | Hours | Deliverable |
|:-----|:------|:------|:------------|
| W26 | Core Metadata | 32 | Rust API, persistence v0.4 |
| W27 | Binary Quantization | 48 | BQ search, SIMD, benchmarks |
| W28 | WASM & Integration | 40 | WASM API, docs |
| W29 | Buffer & Release | 22 | Contingency, polish, v0.6.0 release |

**Base Total:** 140 hours (3.5 weeks)
**With 30% Contingency:** ~182 hours (~4.5 weeks)

**Contingency Allocation:**
- Unforeseen integration issues: 15 hours
- Performance tuning: 7 hours

---

## 10. Open Questions

| Question | Decision Path |
|:---------|:--------------|
| Default storage mode (F32 vs BQ)? | Start with F32, BQ opt-in. Evaluate based on benchmarks. |
| Hybrid filter + BQ priority? | Post-filter on BQ results. Simpler, evaluate ACORN for v0.7. |
| Memory pressure thresholds? | 80% warn, 95% degrade. Test on mobile. |

---

## 11. Approval

| Reviewer | Status | Date |
|:---------|:-------|:-----|
| PLANNER | PROPOSED | 2025-12-20 |
| META_ARCHITECT | APPROVED | 2025-12-20 |
| HOSTILE_REVIEWER | APPROVED | 2025-12-20 |

---

**Document Status:** [APPROVED]
**Review Document:** `docs/reviews/2025-12-20_RFC-002_IMPLEMENTATION_PLAN_APPROVED.md`
**Target Release:** v0.6.0

## Revision History

| Version | Date | Change |
|:--------|:-----|:-------|
| 1.0 | 2025-12-20 | Initial draft (120 hours) |
| 1.1 | 2025-12-20 | Fixed hour totals (140 hours base), added 30% contingency (182 hours total) |


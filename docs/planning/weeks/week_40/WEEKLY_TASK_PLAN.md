# Week 40: Flat Index Implementation

**Date Range:** 2026-02-03 to 2026-02-08
**Focus:** Implement FlatIndex for small datasets with 100% recall
**Hours:** 32h (5-6h/day)
**Status:** [PROPOSED]
**Depends On:** Week 39 COMPLETE (Hybrid Search)
**RFC:** Internal specification (Option A per ROADMAP v6.1 Milestone 9.3)

---

## 1. Context

### Strategic Rationale

Flat Index addresses a genuine gap in EdgeVec's capability matrix:

| Use Case | HNSW | Flat Index |
|:---------|:-----|:-----------|
| Large datasets (>10k) | Optimal | Slow |
| Small datasets (<10k) | Overhead | Optimal |
| Exact recall (100%) | ~95% | 100% |
| Append-heavy workloads | O(log n) | O(1) |
| Memory efficiency | Graph overhead | Vectors only |

**Community Context:** @jsonMartin (first community contributor) announced Flat Index interest in v0.7.0 release. This implementation follows internal specification path per ROADMAP condition gate.

### Roadmap Alignment

- **Milestone:** 9.3 (Flat Index — CONDITIONAL)
- **Version:** v0.9.0
- **Phase:** Implementation Phase
- **Prerequisite:** Week 39 Hybrid Search COMPLETE

---

## 2. Technical Architecture

### FlatIndex Data Structure

```rust
/// O(1) append, brute-force search index for small collections.
pub struct FlatIndex {
    /// Dense vectors in column-major layout
    vectors: Vec<f32>,

    /// Vector dimension
    dim: u32,

    /// Number of vectors (including deleted)
    capacity: u64,

    /// Bitmap for deleted vectors
    deleted: BitSet,

    /// Delete count for cleanup threshold
    delete_count: usize,

    /// Optional BQ quantization
    quantized: Option<Vec<u8>>,

    /// Distance metric
    metric: Metric,
}
```

### Memory Layout (10k vectors, 768D)

| Mode | Storage | Size |
|:-----|:--------|:-----|
| F32 | `10000 * 768 * 4` | ~30MB |
| BQ | `10000 * 768 / 8` | ~1MB |
| Deleted bitmap | `10000 / 8` | ~1.2KB |

### Search Algorithm

```rust
fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
    // 1. Compute distances to all vectors (SIMD-accelerated)
    // 2. Skip deleted vectors via BitSet
    // 3. Maintain top-k min-heap
    // 4. Return sorted results
    // Complexity: O(n * d) ~ 50ms for 10k 768D
}
```

---

## 3. Tasks Overview

| Day | Date | Focus | Hours | Priority |
|:----|:-----|:------|:------|:---------|
| 1 | 2026-02-03 | FlatIndex struct + insert | 5h | P0 |
| 2 | 2026-02-04 | Search + SIMD dispatch | 5h | P0 |
| 3 | 2026-02-05 | Optimization + quantization | 5h | P0 |
| 4 | 2026-02-06 | Persistence + snapshots | 5h | P1 |
| 5 | 2026-02-07 | WASM + TypeScript | 5h | P1 |
| 6 | 2026-02-08 | Testing + Hostile Review | 7h | P0 |

**Total:** 32 hours

---

## 4. Day Summaries

### Day 1: Foundation & Data Structure (5h)

**Focus:** Core FlatIndex type, memory layout, insertion

| Task ID | Description | Hours |
|:--------|:------------|:------|
| W40.1.1 | Create `src/index/flat.rs` with FlatIndex struct | 1.5h |
| W40.1.2 | Implement `insert()` method | 2h |
| W40.1.3 | Unit tests for insertion | 1.5h |

**Exit Criteria:**
- FlatIndex struct compiles with doc comments
- `insert()` allocates IDs, stores vectors correctly
- 5+ unit tests passing

### Day 2: Search Implementation (5h)

**Focus:** Brute-force search with all distance metrics

| Task ID | Description | Hours |
|:--------|:------------|:------|
| W40.2.1 | Implement `search()` method | 2.5h |
| W40.2.2 | Add SIMD dispatch for distance computation | 1.5h |
| W40.2.3 | Unit tests for search | 1h |

**Exit Criteria:**
- `search()` returns correct top-k results
- All 4 metrics work (Cosine, Dot, L2, Hamming)
- 8+ unit tests passing

### Day 3: Optimization & Quantization (5h)

**Focus:** Performance tuning, deletion, optional BQ

| Task ID | Description | Hours |
|:--------|:------------|:------|
| W40.3.1 | Optimize search inner loop | 1.5h |
| W40.3.2 | Implement `soft_delete()` + cleanup | 1.5h |
| W40.3.3 | Add optional BQ quantization | 1.5h |
| W40.3.4 | Benchmarks | 0.5h |

**Exit Criteria:**
- Search <50ms for 10k vectors
- Soft delete works, bitmap persisted
- BQ optional, reduces memory 32x

### Day 4: Persistence & Metadata (5h)

**Focus:** Snapshot format, restoration

| Task ID | Description | Hours |
|:--------|:------------|:------|
| W40.4.1 | Extend snapshot format for FlatIndex | 1.5h |
| W40.4.2 | Implement `to_snapshot()` | 1h |
| W40.4.3 | Implement `from_snapshot()` | 1.5h |
| W40.4.4 | Integration tests | 1h |

**Exit Criteria:**
- Snapshot format versioned (v1.0)
- Round-trip test passes
- Metadata restored correctly

### Day 5: WASM Bindings & TypeScript (5h)

**Focus:** JavaScript API, type definitions

| Task ID | Description | Hours |
|:--------|:------------|:------|
| W40.5.1 | Add WASM bindings | 2h |
| W40.5.2 | TypeScript definitions | 1h |
| W40.5.3 | Example code + documentation | 1h |
| W40.5.4 | Browser integration tests | 1h |

**Exit Criteria:**
- WASM bindings compile (wasm-pack build)
- TypeScript types complete
- Example code runs without errors

### Day 6: Testing, Benchmarks, Hostile Review (7h)

**Focus:** Comprehensive validation, approval

| Task ID | Description | Hours |
|:--------|:------------|:------|
| W40.6.1 | Property-based tests | 1.5h |
| W40.6.2 | Benchmark suite | 1h |
| W40.6.3 | Recall/accuracy tests | 1h |
| W40.6.4 | Integration + compatibility tests | 1h |
| W40.6.5 | Documentation polish | 0.5h |
| W40.6.6 | Hostile review submission | 2h |

**Exit Criteria:**
- 30+ property tests passing
- 100% recall validated
- HOSTILE_REVIEWER APPROVED

---

## 5. Deliverables

### Files Created

```
src/index/
├── mod.rs                     # Module exports
└── flat.rs                    # FlatIndex implementation

benches/
└── flat_bench.rs              # Criterion benchmarks

tests/
├── flat_test.rs               # Unit tests
├── flat_persistence_test.rs   # Snapshot tests
└── flat_recall_test.rs        # Accuracy validation
```

### Files Modified

```
src/lib.rs                     # Add index module
Cargo.toml                     # Add benchmark config
pkg/edgevec.d.ts               # TypeScript definitions
src/wasm/mod.rs                # WASM bindings
```

### Documentation

```
docs/
├── api/FLAT_INDEX.md          # API reference
└── reviews/
    └── 2026-02-08_flat_index_APPROVED.md
```

---

## 6. Risk Register

| ID | Risk | Likelihood | Impact | Mitigation |
|:---|:-----|:-----------|:-------|:-----------|
| R40.1 | Search latency >50ms at 10k | MEDIUM | MEDIUM | SIMD optimization, benchmark early (Day 2) |
| R40.2 | Memory overhead exceeds HNSW | LOW | LOW | Column-major layout, BQ fallback |
| R40.3 | WASM memory limits hit | LOW | MEDIUM | BQ quantization, warn at >100MB |
| R40.4 | Snapshot format conflicts | LOW | MEDIUM | Version check, graceful migration |
| R40.5 | Integration breaks HNSW | LOW | HIGH | Additive module, no existing code changes |

---

## 7. Performance Targets

| Benchmark | Target | Acceptable |
|:----------|:-------|:-----------|
| Search (1k vectors, k=10) | <1ms | <2ms |
| Search (10k vectors, k=10) | <10ms | <50ms |
| Insert (single) | <100us | <200us |
| Insert (1k batch) | <10ms | <20ms |
| Memory (10k 768D, F32) | ~30MB | <50MB |
| Memory (10k 768D, BQ) | ~1MB | <2MB |
| Recall | 100% | 100% |

---

## 8. Exit Criteria

Week 40 is complete when:

- [ ] FlatIndex struct implemented with insert/search
- [ ] All 4 distance metrics supported
- [ ] Search <50ms for 10k vectors
- [ ] 100% recall validated
- [ ] Soft delete with bitmap
- [ ] Optional BQ quantization
- [ ] Persistence (snapshot save/load)
- [ ] WASM bindings functional
- [ ] TypeScript types complete
- [ ] 30+ unit tests passing
- [ ] 20+ property tests passing
- [ ] Benchmarks within targets
- [ ] Clippy clean (0 warnings)
- [ ] HOSTILE_REVIEWER APPROVED

---

## 9. Week Handoff

### To Week 41

**Completed (Week 40):**
- FlatIndex with O(1) insert, brute-force search
- 100% recall guarantee (exhaustive search)
- WASM bindings + TypeScript types
- Persistence format compatible with existing snapshots
- Optional BQ quantization for memory reduction

**Ready for Week 41:**
- v0.9.0 release preparation
- Documentation consolidation
- Performance comparison (Flat vs HNSW)
- Community announcement (@jsonMartin credit)

---

## 10. Commit Message Template

```
feat(index): implement FlatIndex for exact search (Week 40)

- Add FlatIndex with O(1) insert, brute-force search
- Support all 4 distance metrics (Cosine, Dot, L2, Hamming)
- Implement soft delete with BitSet cleanup threshold
- Add optional BQ quantization (32x memory reduction)
- Persistence via snapshot format v1.0
- WASM bindings: createFlatIndex(), flatSearch(), insertFlat()
- TypeScript definitions for FlatIndex API

Performance:
- Search (10k, k=10): Xms (target <50ms)
- Insert: Xus (target <100us)
- Memory (10k 768D, F32): XMB
- Recall: 100% (exhaustive)

Tests:
- 30+ unit tests
- 20+ property tests
- Recall validation passing

Closes #[flat-index-issue]

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

**Agent:** PLANNER
**Hours:** 32h total
**Priority:** P0 (v0.9.0 milestone)
**Status:** [PROPOSED]

# HOSTILE_REVIEWER: RFC_SPARSE_VECTORS.md

**Date:** 2026-01-08
**Artifact:** docs/rfcs/RFC_SPARSE_VECTORS.md (RFC-007)
**Author:** META_ARCHITECT
**Type:** Architecture (RFC/Design Document)
**Verdict:** ✅ APPROVED (with fixes applied)

---

## Review Summary

RFC-007 defines sparse vector support for EdgeVec v0.9.0, enabling hybrid search that combines dense semantic embeddings with sparse keyword features (BM25, TF-IDF).

The RFC is **fundamentally sound** with a well-researched design based on industry standards (sprs CsVec, Milvus). All identified issues were addressable without architectural redesign.

---

## Issues Found and Resolved

### Critical Issues (3) — ALL FIXED

| ID | Issue | Location | Fix Applied |
|:---|:------|:---------|:------------|
| C1 | Complexity analysis incorrect (O(min) vs O(sum)) | Line 170 | Changed to "O(\|a\| + \|b\|) worst case" |
| C2 | Missing 1M vector scale calculation | Lines 227-235 | Added full 1M estimate (~408 MB) |
| C3 | `insertHybrid` API used but undefined | Line 699 | Added to TypeScript API (Lines 311-313) |

### Major Issues (4) — ALL FIXED

| ID | Issue | Location | Fix Applied |
|:---|:------|:---------|:------------|
| M1 | No P50/P99 performance targets | Lines 449-458 | Split into P50/P99 columns |
| M2 | Empty vector validation vs constructor | Lines 361, 560 | Removed `empty()`, added `singleton()` |
| M3 | VectorId/SparseId namespace collision | Line 202 | Added unified ID documentation |
| M4 | WeightedSum fusion incomplete | Lines 334-343 | Added full min-max normalization |

### Minor Issues (3) — ALL FIXED

| ID | Issue | Location | Fix Applied |
|:---|:------|:---------|:------------|
| m1 | Inverted index posting lists unbounded | Line 246 | Added MAX_POSTING_LENGTH cap |
| m2 | Phase timeline overlap | Line 504 | Shifted phases by 1 week |
| m3 | SearchResult ID mapping unclear | Line 287 | Added unified ID comment |

---

## Verification

### Completeness Checklist

- [x] All core components defined (SparseVector, SparseStorage, HybridStorage, Metrics)
- [x] Data structures sized (8 bytes/element CSR)
- [x] WASM boundary specified (TypeScript API complete)
- [x] Performance budget with P50/P99 targets
- [x] Memory calculations for 100k AND 1M vectors
- [x] 1M sparse fits Safari ~1GB limit (~408 MB)

### Consistency Checklist

- [x] Aligns with ARCHITECTURE.md patterns
- [x] VectorId/SparseId namespace documented
- [x] API consistent across Rust and TypeScript sections
- [x] `insertHybrid` defined and used correctly

### Feasibility Checklist

- [x] Timeline realistic (Weeks 37-42)
- [x] Complexity analysis correct
- [x] Performance targets achievable (similar to competitors)

---

## Strengths of RFC

1. **Industry alignment**: CSR format matches sprs, Milvus, Qdrant
2. **Comprehensive API**: Both Rust and TypeScript fully specified
3. **Hybrid search**: RRF and WeightedSum fusion documented
4. **Reuses codebase patterns**: Metric trait, storage layout, BitVec deletion
5. **Clear phase breakdown**: 5 phases with specific deliverables

---

## Implementation Recommendations

1. **Start with `SparseVector` and metrics** — Foundation for everything else
2. **Property tests critical** — Commutativity, associativity, normalization bounds
3. **Benchmark early** — Validate <500ns dot product target in Week 37
4. **Inverted index optional** — Brute force sufficient for <100k vectors

---

## Gate Status

This RFC approval **does not unlock a gate** — it is a design document for v0.9.0 features. Implementation requires additional planning and code review.

---

## VERDICT

```
┌─────────────────────────────────────────────────────────────────────┐
│   HOSTILE_REVIEWER: APPROVED                                        │
│                                                                     │
│   Artifact: RFC_SPARSE_VECTORS.md (RFC-007)                         │
│   Author: META_ARCHITECT                                            │
│                                                                     │
│   Critical Issues: 3 → 0 (all fixed)                                │
│   Major Issues: 4 → 0 (all fixed)                                   │
│   Minor Issues: 3 → 0 (all fixed)                                   │
│                                                                     │
│   UNLOCK: Phase 1 implementation may proceed (Week 37)              │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

**Reviewed by:** HOSTILE_REVIEWER
**Authority:** ULTIMATE VETO POWER
**Date:** 2026-01-08

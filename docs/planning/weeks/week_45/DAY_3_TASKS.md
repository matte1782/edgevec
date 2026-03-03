# Week 45 — Day 3 Tasks (Wednesday, Mar 26)

**Date:** 2026-03-26
**Focus:** Product Quantization Literature Review
**Agent:** META_ARCHITECT + BENCHMARK_SCIENTIST
**Status:** PENDING

---

## Day Objective

Conduct a thorough literature review of Product Quantization (PQ) techniques to answer the three research questions from the ROADMAP (Milestone 10.4). This is the research phase — no implementation. The goal is to produce a data-backed document that informs the W46 GO/NO-GO decision.

**Success Criteria:**
- Literature review document complete with cited sources
- All 3 research questions addressed with data
- WASM feasibility assessment with concrete constraints
- Preliminary GO/NO-GO recommendation (final in W46 after benchmarks)

---

## Context

**From ROADMAP Milestone 10.4:**
> Research Questions (must answer ALL):
> 1. Is 64x compression worth the complexity vs BQ's 32x?
> 2. Can we achieve <100ns lookup overhead in WASM?
> 3. How does recall compare: PQ vs BQ at same memory budget?

**Current EdgeVec State:**
- Binary Quantization (BQ): 32x compression, implemented since v0.6.0
- SQ8: 4x compression, implemented since v0.2.0
- Memory per vector (768D): ~100 bytes with BQ
- Target: <100 bytes including index overhead

---

## Tasks

### W45.3a: Survey PQ Implementations (2h)

**Description:** Study how major vector databases implement Product Quantization to understand practical trade-offs.

**Systems to Survey:**
1. **Faiss (Meta)** — The reference PQ implementation
   - Training algorithm (k-means per subspace)
   - Distance computation (ADC vs SDC)
   - Memory layout and lookup tables
2. **ScaNN (Google)** — Anisotropic quantization variant
   - How it differs from standard PQ
   - Reported recall vs compression trade-offs
3. **Qdrant** — Production Rust PQ implementation
   - Rust-specific patterns and performance
   - Memory-mapped codebooks
   - Rescoring strategy
4. **Weaviate / Milvus** — Alternative approaches
   - Product Quantization with HNSW integration
   - Training data requirements

**Deliverable:** Section 1-2 of `docs/research/PRODUCT_QUANTIZATION_LITERATURE.md`

**Acceptance:**
- [ ] 4 systems surveyed with key design decisions documented
- [ ] Compression ratios and recall numbers cited from published benchmarks
- [ ] Training requirements documented (data size, compute time)

### W45.3b: PQ vs BQ Trade-off Analysis (1.5h)

**Description:** Create a quantitative comparison of PQ vs BQ at equivalent memory budgets.

**Analysis Dimensions:**
| Dimension | BQ (Current) | PQ (Expected) |
|:----------|:-------------|:--------------|
| Compression | 32x | 64x (8 subspaces, 256 centroids) |
| Recall @10 | >0.95 (measured) | ~0.90-0.97 (from literature) |
| Memory/vector (768D) | ~96 bytes | ~48 bytes |
| Index build time | O(N) | O(N * k * iterations) |
| Query overhead | Hamming + rescore | ADC lookup tables |
| Implementation complexity | LOW | HIGH |

**Key Question:** Does 2x additional compression justify the complexity when BQ already achieves <100 bytes/vector?

**Deliverable:** Section 3 of `PRODUCT_QUANTIZATION_LITERATURE.md`

**Acceptance:**
- [ ] Side-by-side comparison table with cited numbers
- [ ] Clear statement on when PQ beats BQ (what scale?)
- [ ] Memory savings calculation for 100K, 500K, 1M vectors

### W45.3c: WASM Feasibility Assessment (1.5h)

**Description:** Evaluate whether PQ can be practically implemented in WASM with acceptable performance.

**Constraints to Evaluate:**
1. **Codebook Training in Browser:**
   - k-means requires multiple passes over training data
   - 768D vectors, 8 subspaces (96D each), 256 centroids
   - Estimated training time for 10K, 50K, 100K vectors
   - Can training be done offline and codebook imported?

2. **Lookup Table Memory:**
   - ADC requires 256 * num_subspaces * sizeof(f32) per query
   - 256 * 8 * 4 = 8KB per active query — fits in L1 cache
   - Multiple simultaneous queries? Memory pressure?

3. **SIMD Acceleration:**
   - Can WASM SIMD128 accelerate ADC lookups?
   - Gather operations not available in SIMD128 — sequential lookups
   - Impact on the <100ns target

4. **Bundle Size:**
   - Total codebook storage: 256 centroids * 8 subspaces * 96 floats/centroid * 4 bytes/float = 768KB (all subspaces combined, single codebook set)
   - Too large for browser? Can be lazy-loaded from IndexedDB?

**Deliverable:** Section 4 of `PRODUCT_QUANTIZATION_LITERATURE.md`

**Acceptance:**
- [ ] Training time estimates for browser environment
- [ ] Memory budget for lookup tables
- [ ] SIMD acceleration feasibility assessment
- [ ] Bundle/storage size impact documented

### W45.3d: Draft Research Questions and Exit Criteria (1h)

**Description:** Based on the literature review, refine the research questions and define concrete exit criteria for the W46 GO/NO-GO decision.

**Deliverable:** Section 5 of `PRODUCT_QUANTIZATION_LITERATURE.md`

**Content:**
1. Refined research questions (based on what we learned)
2. Specific benchmarks needed in W46
3. GO/NO-GO criteria (quantitative thresholds)
4. Preliminary recommendation with confidence level
5. If GO: high-level implementation plan for W46-47
6. If NO-GO: what conditions would change the decision

**Acceptance:**
- [ ] Exit criteria are binary pass/fail (no subjective judgments)
- [ ] Benchmarks are specific and reproducible
- [ ] Implementation plan (if GO) fits within W46-47 budget (16h)

---

## Day 3 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~6h |
| New files | 1 (`PRODUCT_QUANTIZATION_LITERATURE.md`) |
| Research questions answered | 3 (preliminary) |
| Systems surveyed | 4 |

---

## Handoff

After Day 3 completion:
- **Status:** PENDING_HOSTILE_REVIEW (bundled with Day 5 review)
- **Next:** Day 4 — PQ Benchmark Design + API Stability Audit
- **Artifacts:** `docs/research/PRODUCT_QUANTIZATION_LITERATURE.md`

---

**END OF DAY 3 TASKS**

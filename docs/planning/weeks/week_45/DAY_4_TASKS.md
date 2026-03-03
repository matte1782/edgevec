# Week 45 — Day 4 Tasks (Thursday, Mar 27)

**Date:** 2026-03-27
**Focus:** PQ Benchmark Design + API Stability Audit
**Agent:** BENCHMARK_SCIENTIST + META_ARCHITECT
**Status:** PENDING

---

## Day Objective

Design the benchmark methodology for W46 PQ implementation testing. Conduct an API stability audit across all EdgeVec public surfaces (Rust, WASM, TypeScript) to identify breaking change candidates before v1.0.

**Success Criteria:**
- PQ benchmark plan ready for W46 execution
- API surface inventory complete (Rust + WASM + TS)
- Breaking change candidates identified and documented
- API stability recommendations drafted

---

## Tasks

### W45.4a: PQ Benchmark Methodology Design (2h)

**Description:** Design reproducible benchmarks that will definitively answer the three PQ research questions in W46.

**Benchmark Suite:**

1. **Compression Benchmark:**
   - Dataset: Random 768D vectors (10K, 50K, 100K)
   - Measure: Compressed size per vector (PQ vs BQ vs SQ8 vs raw)
   - Expected: PQ ~48 bytes/vec vs BQ ~96 bytes/vec

2. **Recall Benchmark:**
   - Dataset: Same as above, with ground truth (brute-force k=10)
   - Measure: Recall@10 after quantization
   - Compare: PQ recall vs BQ recall at same memory budget
   - Expected: PQ >=0.90 recall with 64x compression

3. **Latency Benchmark:**
   - Measure: Per-query lookup overhead (ADC distance computation)
   - Target: <100ns lookup overhead
   - Compare: PQ ADC vs BQ Hamming vs raw f32 distance
   - Environment: Both native (cargo bench) and WASM (browser)

4. **Training Benchmark:**
   - Measure: Codebook training time
   - Variables: N vectors, k centroids (256), subspaces (8), iterations (10, 20, 50)
   - Environment: WASM in Chrome + native Rust
   - Acceptance: Training for 100K vectors < 30s in browser

**Deliverable:** `docs/research/PQ_BENCHMARK_PLAN.md`

**Acceptance:**
- [ ] All 4 benchmark types defined with methodology
- [ ] Expected results documented (for comparison)
- [ ] Hardware/environment requirements specified
- [ ] Results format templated (tables, charts)

### W45.4b: API Surface Inventory (2h)

**Description:** Create a complete inventory of all public APIs across Rust, WASM, and TypeScript surfaces.

**Scope:**

1. **Rust Public API (`src/lib.rs` + re-exports):**
   - All `pub` types, traits, functions, methods
   - All `pub use` re-exports
   - Feature-gated APIs (`#[cfg(feature = ...)]`)

2. **WASM API (`src/wasm/` + `pkg/`):**
   - All `#[wasm_bindgen]` exported functions
   - TypeScript type definitions in `pkg/*.d.ts`
   - JavaScript wrapper API in `pkg/edgevec-wrapper.js`

3. **edgevec-langchain API (`pkg/langchain/src/`):**
   - All exported types, classes, functions
   - `EdgeVecStore` public methods
   - `initEdgeVec` and configuration types

**Deliverable:** `docs/audits/API_SURFACE_INVENTORY.md`

**Format per API:**
```
| API | Surface | Stability | Breaking Change Risk | Notes |
```

**Acceptance:**
- [ ] All public Rust APIs listed
- [ ] All WASM exports listed
- [ ] All TypeScript exports listed
- [ ] Each API has stability assessment (STABLE, UNSTABLE, EXPERIMENTAL)

### W45.4c: Breaking Change Candidates (1h)

**Description:** Based on the API inventory, identify APIs that may need breaking changes before v1.0 freeze.

**Categories:**
1. **Rename candidates** — APIs with inconsistent naming
2. **Signature changes** — APIs that should accept different params
3. **Deprecation candidates** — APIs that should be removed
4. **Type changes** — APIs that return wrong types or use `any`
5. **Error handling** — APIs that panic instead of returning Result

**Deliverable:** Section in `docs/audits/API_STABILITY_AUDIT.md`

**Acceptance:**
- [ ] Each candidate has rationale and proposed change
- [ ] Impact assessment (how many users affected)
- [ ] Priority: P0 (must change), P1 (should change), P2 (nice to change)

### W45.4d: API Stability Recommendations (1h)

**Description:** Write recommendations for the v1.0 API freeze based on the audit.

**Content:**
1. **Recommended v1.0 public API surface** — What to include/exclude
2. **Deprecation plan** — What to deprecate before v1.0
3. **Semantic versioning policy** — How to handle future changes
4. **TypeScript type stability** — Freezing `.d.ts` interfaces
5. **WASM ABI stability** — Ensuring binary compatibility

**Deliverable:** `docs/audits/API_STABILITY_AUDIT.md` (main document)

**Acceptance:**
- [ ] Clear recommendation for each breaking change candidate
- [ ] Timeline for implementing changes (which week)
- [ ] v1.0 API freeze criteria defined

---

## Day 4 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~6h |
| New files | 3 (PQ_BENCHMARK_PLAN.md, API_SURFACE_INVENTORY.md, API_STABILITY_AUDIT.md) |
| APIs audited | All public surfaces |
| Benchmark types designed | 4 |

---

## Handoff

After Day 4 completion:
- **Status:** PENDING_HOSTILE_REVIEW (all W45 artifacts ready)
- **Next:** Day 5 — Hostile Review of all artifacts
- **Artifacts:** PQ benchmark plan, API inventory, API stability audit

---

**END OF DAY 4 TASKS**

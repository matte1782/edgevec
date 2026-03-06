# Week 47 — Day 5 Tasks (Friday, Apr 11)

**Date:** 2026-04-11
**Focus:** B4 PQ vs BQ Comparison + Final GO/NO-GO Update + End-of-Week Hostile Review
**Agents:** BENCHMARK_SCIENTIST, PLANNER, RUST_ENGINEER, HOSTILE_REVIEWER
**Status:** PENDING

---

## Day Objective

Complete the B4 comparative benchmark, update the GO/NO-GO decision document with all W47 results, and pass the end-of-week hostile review. Also address LOW-priority carry-forward items if time permits.

**Success Criteria:**
- B4 PQ vs BQ+rescore comparison documented on real embeddings
- PQ_GO_NOGO_DECISION.md updated — all 6 gates have definitive PASS/FAIL
- End-of-week hostile review GO
- LOW items (lib.rs re-exports, dimensions=0 validation) addressed

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] Day 4 time tracking — G3/G4 results
- [ ] `src/quantization/binary.rs` — BQ API: `BinaryQuantizer::quantize_batch()`
- [ ] `src/similarity.rs` — `hamming_distance()` function
- [ ] `docs/benchmarks/PQ_GO_NOGO_DECISION.md` — document to update
- [ ] Day 4 recall results — G3 number and query set for B4 reuse

---

## Tasks

### W47.5a: B4 PQ vs BQ+Rescore Comparison (2h) — BENCHMARK_SCIENTIST

**Dependency:** W47.4c complete (G3 results and query set available)

**Protocol (using same 50K real embeddings and 100 queries from W47.4c):**

1. **PQ recall@10:** Reuse result from W47.4c
2. **BQ+rescore pipeline:**
   - Binarize all 50K vectors via `BinaryQuantizer::quantize_batch()` from `src/quantization/binary.rs`
   - For each query: compute Hamming distance to all 50K BQ vectors via `hamming_distance()` from `src/similarity.rs`
   - Select top-100 by Hamming distance
   - Rescore top-100 using original f32 vectors with L2 distance
   - Select top-10 from rescored results
3. **Recall@10:** compare BQ+rescore top-10 against brute-force L2 ground truth (same as W47.4c)

**Memory budget:** 50K x 768D x f32 = 147MB + 50K x 96B BQ = 4.6MB = ~152MB total. Fits in 32GB RAM.

**Commands:**
```bash
cargo test test_b4_bq_rescore_comparison -- --nocapture
```

**Expected Output:**
```
B4 Comparison (50K real embeddings, 768D, 100 queries):
  PQ (M=8):       recall@10 = X.XX
  BQ+rescore(100): recall@10 = Y.YY
  Winner: [PQ | BQ+rescore] by Z.ZZ points
  Latency: PQ=A ms, BQ+rescore=B ms per query
```

**Acceptance:**
- [ ] Both recall numbers use same query set and ground truth
- [ ] BQ+rescore pipeline documented (Hamming top-100 → f32 L2 top-10)
- [ ] Memory usage confirmed < 200MB
- [ ] Comparison is fair: same data, same queries, same ground truth

---

### W47.5b: Update PQ_GO_NOGO_DECISION.md (2h) — PLANNER + BENCHMARK_SCIENTIST

**Dependency:** W47.3a (G2), W47.4b (G4), W47.4c (G3), W47.5a (B4) all complete

**Required Updates:**
Add "## W47 Validation Results" section containing:
1. **G2 WASM result:** per-candidate latency, Chrome version, PASS/FAIL
2. **G3 real-embedding result:** recall@10, model, dataset, PASS/FAIL
3. **G4 optimization result:** native time, WASM time, optimizations applied, PASS/FAIL
4. **B4 comparison:** PQ vs BQ+rescore on real data
5. **Updated gate summary table:** all 6 gates with final PASS/FAIL (NO more INCONCLUSIVE or UNTESTED)
6. **Final verdict:** GO, NO-GO, or CONDITIONAL with documented limitations

**Decision Tree for Final Verdict:**
- All 6 gates PASS → **GO** (PQ is production-ready)
- G1+G2+G5+G6 PASS, G3 or G4 CONDITIONAL → **CONDITIONAL GO** with documented requirements (e.g., "M=16 required" or "training limited to 50K")
- Any gate hard FAIL with no mitigation → **NO-GO** with explicit feature limitations

**Commands:**
```bash
# Edit the document
# Then verify consistency
grep -c "PASS\|FAIL" docs/benchmarks/PQ_GO_NOGO_DECISION.md
# Should show all 6 gates with definitive verdicts
```

**Acceptance:**
- [ ] All 6 gates have PASS or FAIL (no INCONCLUSIVE/UNTESTED)
- [ ] W47 WASM benchmark environment documented
- [ ] Real-embedding results included with model and dataset info
- [ ] Training optimization measurements with individual contributions
- [ ] Final verdict follows logically from gate results

---

### W47.5c: LOW — PQ Types in lib.rs Re-Exports (0.5h) — RUST_ENGINEER

**Dependency:** None

**Implementation:**
Add to `src/lib.rs`:
```rust
pub use quantization::product::{PqCodebook, PqCode, DistanceTable, PqSearchResult, PqError};
```

**Commands:**
```bash
cargo build    # Verify compilation
cargo doc      # Verify docs generate
```

**Acceptance:**
- [ ] `cargo build` succeeds
- [ ] PQ types accessible from `edgevec::PqCodebook` etc.

---

### W47.5d: LOW — dimensions=0 Validation (0.5h) — RUST_ENGINEER

**Dependency:** None

**Implementation:**
In `PqCodebook::train()`, add early check for `dimensions == 0`.

**Commands:**
```bash
cargo test test_train_zero_dimensions
```

**Acceptance:**
- [ ] `test_train_zero_dimensions` returns appropriate error, no panic

---

### W47.5e: END-OF-WEEK HOSTILE REVIEW (2h) — HOSTILE_REVIEWER

**Dependency:** W47.5b complete

**Scope:** Updated `docs/benchmarks/PQ_GO_NOGO_DECISION.md` + G3/G4 results

**Attack Vectors:**
1. G3 recall uses real embeddings (all-mpnet-base-v2, 768D), NOT synthetic uniform
2. G4 optimization actually achieves targets (native <30s, WASM <60s)
3. All 6 gates have definitive verdicts with specific numbers
4. No cherry-picked results — negative results prominently documented
5. Training optimization doesn't degrade recall (before/after comparison)
6. B4 comparison is fair (same data, queries, ground truth)
7. Decision logic follows from data

**Decision Tree:**
- **GO** → Day 6 proceeds to commit
- **CONDITIONAL GO** → fix minor issues, Day 6 proceeds
- **NO-GO** → HALT commit. Fix criticals + majors, resubmit Round 2

**Acceptance:**
- [ ] 0 Critical findings
- [ ] 0 Major findings
- [ ] Review document created in `docs/reviews/`

---

## Day 5 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~7h |
| B4 comparison | Complete |
| GO/NO-GO document | All 6 gates definitive |
| Hostile review | GO verdict |
| LOW items | 2 completed (re-exports + dims=0) |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W47.5a | 2h | | |
| W47.5b | 2h | | |
| W47.5c | 0.5h | | |
| W47.5d | 0.5h | | |
| W47.5e | 2h | | |
| **Total** | **7h** | | |

---

## Handoff to Day 6 (EXTRA DETAILED — Weekend Gap)

**This handoff must survive a 2-day weekend gap. Be explicit.**

**Codebase state at EOD Friday:**
- `src/quantization/product.rs` — PQ with early-stop, reduced iters, parallel subspaces
- `src/wasm/mod.rs` — 3 PQ WASM exports (train_pq, encode_pq, pq_search)
- `src/lib.rs` — PQ types re-exported
- `docs/benchmarks/PQ_GO_NOGO_DECISION.md` — updated with W47 results, all 6 gates definitive
- `tests/wasm/pq_bench.html` + `pq_bench.js` — WASM benchmark harness
- `tests/data/embeddings_768d_50k.bin` — real embeddings (gitignored)
- `tests/data/generate_embeddings.py` — reproducible generation script
- Two hostile review reports in `docs/reviews/`

**What passed hostile review:**
- Mid-week: WASM exports + G2 (verdict: ___)
- End-of-week: G3/G4/GO-NO-GO (verdict: ___)

**Outstanding hostile review findings to fix on Day 6:**
- List ALL findings here after review completes
- ___ (fill in after W47.5e)

**Git status:**
- All changes are local (not committed yet)
- No pending pushes expected

**Day 6 focus:** Fix review findings, update ROADMAP, update CHANGELOG, commit, create gate file

**First action Monday morning:** Re-read this handoff + the hostile review reports in `docs/reviews/`

---

**END OF DAY 5 TASKS**

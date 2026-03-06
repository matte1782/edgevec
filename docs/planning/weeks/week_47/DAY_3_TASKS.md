# Week 47 — Day 3 Tasks (Wednesday, Apr 9)

**Date:** 2026-04-09
**Focus:** G2 WASM ADC Benchmark + Training Optimization (Early-Stop, Reduce Iters) + Mid-Week Hostile Review
**Agents:** BENCHMARK_SCIENTIST, RUST_ENGINEER, HOSTILE_REVIEWER
**Status:** PENDING

---

## Day Objective

Two parallel tracks:
- **Track B:** Run G2 WASM ADC benchmark at 100K scale via Playwright
- **Track C:** Implement first two training optimizations (early-stop + reduced iterations)

Track C is INDEPENDENT of Track B — it modifies `product.rs`, not `wasm/mod.rs`. Both can proceed in parallel.

Mid-week hostile review scopes ONLY to WASM exports + G2 result (Track B). Track C is unaffected by review verdict.

**Success Criteria:**
- G2: WASM ADC latency < 150ns/candidate P99 measured and documented
- Early-stop optimization implemented with measured improvement over 198.7s baseline
- Reduced-iterations optimization measured
- Hostile review GO on WASM exports + G2

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `tests/wasm/pq_bench.js` — benchmark harness from Day 2
- [ ] `docs/research/PQ_BENCHMARK_PLAN.md` Section 1.3 — reproducibility protocol (warmup, iterations, P99 check)
- [ ] `docs/benchmarks/PQ_GO_NOGO_DECISION.md` — W46 baseline: B2 native 37.6 ns, B7 198.7s
- [ ] `src/quantization/product.rs` — k-means training loop (find convergence check insertion point)
- [ ] `.claude/CLAUDE.md` line 367 — G2 threshold: <150ns/candidate WASM P99

---

## Tasks

### W47.3a: G2 WASM ADC Benchmark (2h) — BENCHMARK_SCIENTIST

**Dependency:** Day 2 complete (Playwright harness verified)

**Benchmark Protocol (per PQ_BENCHMARK_PLAN Section 1.3):**
1. Generate 100K synthetic vectors (seed=42, 768D, uniform [-1,1])
2. Train codebook in WASM (M=8, Ksub=256, max_iters=15)
3. Encode all 100K vectors to PQ codes
4. 3 warmup iterations (discarded)
5. 10 measured iterations: for each, run 10 queries over 100K codes
6. Throughput counter: `Elements(10 * 100000)` per iteration (lesson #65)
7. Report median and P99 per-candidate latency in nanoseconds
8. If P99 > 3x median → investigate and re-run per Section 1.3 item 5

**Commands (Playwright):**
```
browser_navigate → http://localhost:8080/tests/wasm/pq_bench.html
browser_evaluate → benchADC(100000, 10, 10)    # 100K codes, 10 queries, 10 iters
browser_evaluate → navigator.userAgent           # Chrome version
browser_console_messages                         # Check for errors
```

**Expected Output:**
```json
{
  "median_ns_per_candidate": <number>,
  "p99_ns_per_candidate": <number>,
  "total_ms": <number>,
  "iterations": 10,
  "warmup": 3,
  "codes": 100000,
  "queries": 10
}
```

**Decision Tree:**
- If per-candidate P99 < 150ns → **G2 PASS**
- If per-candidate P99 >= 150ns → investigate V8 deoptimization; try `wasm-opt -O3`; if still fails → **G2 FAIL**, document
- If Chrome tab crashes at 100K → reduce to 50K, extrapolate, document

**Acceptance:**
- [ ] Per-candidate latency < 150ns WASM P99
- [ ] Hardware + Chrome version documented
- [ ] 10 measured iterations, 3 warmup
- [ ] P99 <= 3x median (no anomalous outliers)

---

### W47.3b: Training Optimization #1 — Early-Stop (2h) — RUST_ENGINEER

**Dependency:** None (Track C, independent of Track B)

**Implementation in `src/quantization/product.rs`:**
- Add `convergence_threshold: f32` parameter to `PqCodebook::train()` (default: `1e-4`)
- After each k-means iteration, compute max centroid movement
- If movement < threshold, halt early
- Log iteration count at which convergence occurred

**Commands:**
```bash
cargo test test_train_early_stop    # New test
cargo bench --bench pq_bench        # Reuse existing B7 benchmark
# Record: 100K training time WITH early-stop vs baseline 198.7s
```

**Expected Output:**
- New test `test_train_early_stop_converges` passes
- Measured improvement: e.g., "198.7s → Xs (Y% improvement, stopped at iteration Z/15)"

**Acceptance:**
- [ ] `test_train_early_stop_converges` passes
- [ ] 100K training time measured and documented
- [ ] Improvement percentage calculated vs 198.7s baseline
- [ ] No regression in existing PQ tests

---

### W47.3c: Training Optimization #2 — Reduced Iterations (1h) — RUST_ENGINEER

**Dependency:** W47.3b complete (early-stop already implemented)

**Measurement:**
- Run 100K training with `max_iters=5` + early-stop (convergence_threshold=1e-4)
- Compare vs baseline (198.7s) and vs early-stop-only

**Commands:**
```bash
# Modify bench or run dedicated timing test
cargo bench --bench pq_bench        # With max_iters=5
# Also measure recall impact at 10K scale:
cargo test test_recall_reduced_iters  # Compare M=8 recall at iters=5 vs iters=15
```

**Expected Output:**
- Training time in seconds (100K, iters=5 + early-stop)
- Recall@10 comparison at 10K: iters=5 vs iters=15

**Acceptance:**
- [ ] 100K training time measured and documented
- [ ] Recall impact documented (iters=5 vs iters=15)
- [ ] Improvement percentage vs baseline

---

### W47.3d: MID-WEEK HOSTILE REVIEW (1.5h) — HOSTILE_REVIEWER

**Dependency:** W47.3a complete (G2 result needed)

**Scope:** WASM PQ exports in `src/wasm/mod.rs` + G2 WASM benchmark result ONLY. Training optimization (Track C) is NOT in scope.

**Attack Vectors:**
1. WASM exports follow existing handle/opaque-pointer pattern
2. No `unwrap()` in WASM code — all `Result` with `.map_err()`
3. G2 methodology matches PQ_BENCHMARK_PLAN reproducibility protocol
4. Benchmark is reproducible (seed, warmup, iterations documented)
5. Error handling is complete (all 3 functions return `Result`)
6. WASM build still succeeds, existing tests unbroken

**Decision Tree:**
- **GO** → Day 4 proceeds normally
- **CONDITIONAL GO** → fix minor issues Day 3 evening, Day 4 proceeds
- **NO-GO** → HALT Day 4 Track B/D tasks. Fix criticals + majors, resubmit Round 2. Track C (optimization) can continue since it's independent

**Acceptance:**
- [ ] 0 Critical findings
- [ ] 0 Major findings
- [ ] Review document created in `docs/reviews/`

---

## Day 3 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~6.5h |
| G2 verdict | PASS or FAIL (definitive) |
| Training optimizations | 2 (early-stop + reduced iters) |
| Hostile review | GO verdict on WASM + G2 |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W47.3a | 2h | | |
| W47.3b | 2h | | |
| W47.3c | 1h | | |
| W47.3d | 1.5h | | |
| **Total** | **6.5h** | | |

---

## Handoff to Day 4

**Codebase state at EOD:**
- G2 WASM result measured and documented
- Early-stop + reduced-iters optimizations in `product.rs`
- Mid-week hostile review complete
- All tests still passing

**Day 4 prerequisites satisfied:**
- [ ] G2 has definitive PASS/FAIL
- [ ] Hostile review GO (or Track C can proceed regardless)
- [ ] Early-stop and reduced-iters measured (baselines for Day 4 parallel optimization)

**Day 4 focus:** Parallel subspace training (rayon) + G4 combined measurement + G3 real-embedding recall

---

**END OF DAY 3 TASKS**

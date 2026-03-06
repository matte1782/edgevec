# Week 47 — Day 4 Tasks (Thursday, Apr 10)

**Date:** 2026-04-10
**Focus:** Parallel Subspace Training + G4 Final Measurement + G3 Real-Embedding Recall
**Agents:** RUST_ENGINEER, BENCHMARK_SCIENTIST, TEST_ENGINEER
**Status:** PENDING

---

## Day Objective

Complete training optimization with parallel subspaces (rayon), measure combined G4 result on native AND WASM, and validate G3 recall on real embeddings. By EOD, all three open gates (G2, G3, G4) should have definitive PASS/FAIL.

**Success Criteria:**
- G4 native: 100K training < 30s with all optimizations combined
- G4 WASM: 100K training < 60s (or documented crash/extrapolation)
- G3: recall@10 > 0.90 on real embeddings (50K, 768D, M=8 — or M=16 if mitigation needed)
- WASM bundle size increase < 5% after rayon addition
- All regression tests pass

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] Day 3 time tracking — actual hours for W47.3b/W47.3c (optimization baselines)
- [ ] `src/quantization/product.rs` — k-means training loop with early-stop from Day 3
- [ ] `tests/data/embeddings_768d_50k.bin` — verify file exists (generated Day 1)
- [ ] `docs/research/PQ_BENCHMARK_PLAN.md` Section 6.2 — G3 mitigation (M=16)
- [ ] `docs/research/PQ_BENCHMARK_PLAN.md` Section 4.9 — G4 thresholds: native <30s, WASM <60s

**Pre-task measurement:**
```bash
# Record WASM bundle baseline BEFORE rayon changes
wasm-pack build --release
ls -la pkg/edgevec_bg.wasm    # Record size in bytes
```

---

## Tasks

### W47.4a: Training Optimization #3 — Parallel Subspaces (3h) — RUST_ENGINEER

**Dependency:** W47.3b, W47.3c complete (early-stop + reduced iters in place)

**Implementation:**
- Add `rayon` to `Cargo.toml` dependencies (behind `parallel` feature flag)
- Gate with `#[cfg(not(target_arch = "wasm32"))]` — WASM stays single-threaded
- In `PqCodebook::train()`, train M subspaces concurrently via `rayon::par_iter()`
- Use per-subspace deterministic seeding: `ChaCha8Rng::seed_from_u64(base_seed + subspace_index as u64)`

**Commands:**
```bash
# Implementation
cargo build --features parallel    # Native with rayon
cargo test --features parallel     # All tests with rayon

# Determinism test
cargo test test_parallel_train_deterministic --features parallel

# WASM bundle size check (M4 fix)
wasm-pack build --release
ls -la pkg/edgevec_bg.wasm    # Compare with baseline from pre-task
# Size increase must be < 5%

# Combined training time (all 3 optimizations)
cargo bench --bench pq_bench --features parallel    # 100K training
```

**Decision Tree:**
- If `test_parallel_train_deterministic` fails → per-subspace seeding is wrong; debug seed propagation
- If WASM bundle increases > 5% → rayon is leaking into WASM; check `cfg` gates
- If parallel doesn't improve enough → combine with mini-batch k-means as additional strategy

**Acceptance:**
- [ ] `test_parallel_train_deterministic` passes — same seed produces identical codebook
- [ ] `wasm-pack build --release` succeeds
- [ ] WASM bundle size increase < 5%
- [ ] 100K training time measured with all 3 optimizations combined
- [ ] Improvement documented vs baseline (198.7s) and vs early-stop-only

---

### W47.4b: G4 Final Measurement — Native + WASM (2h) — BENCHMARK_SCIENTIST

**Dependency:** W47.4a complete

**Native measurement:**
```bash
# 3 runs minimum, report median
cargo bench --bench pq_bench --features parallel -- training_100k
# Target: median < 30s (G4 native threshold)
```

**WASM measurement (via Playwright or manual):**
```
# WASM uses early-stop + reduced iters ONLY (no rayon)
browser_evaluate → benchTraining(100000, 3)    # 100K, 3 iterations
# Target: median < 60s (G4 WASM threshold)
```

**Decision Tree:**
- If native < 30s AND WASM < 60s → **G4 PASS**
- If native < 30s but WASM > 60s → **G4 CONDITIONAL** (native PASS, WASM needs Web Workers)
- If native > 30s → document all optimization measurements, escalate
- If Chrome crashes at 100K WASM → reduce to 50K, extrapolate per PQ_BENCHMARK_PLAN Section 6.5

**Expected Output:**
```
G4 Native: X.Xs (3 runs: [a, b, c]) — PASS/FAIL vs <30s
G4 WASM:   Y.Ys (3 runs: [a, b, c]) — PASS/FAIL vs <60s
Optimizations applied: early-stop (1e-4) + max_iters=5 + parallel (native only)
```

**Acceptance:**
- [ ] Native median < 30s documented (3 runs)
- [ ] WASM median < 60s documented (3 runs, or extrapolated with justification)
- [ ] All optimization contributions individually documented

---

### W47.4c: G3 Real-Embedding Recall Validation (3h) — BENCHMARK_SCIENTIST

**Dependency:** W47.1a complete (embeddings exist from Day 1)

**Protocol:**
1. Load `tests/data/embeddings_768d_50k.bin` (50K x 768D)
2. Train PQ codebook on all 50K embeddings (M=8, Ksub=256)
3. Encode all 50K vectors to PQ codes
4. Sample 100 random queries (seed=42) from the dataset
5. Compute brute-force L2 ground truth top-10 for each query
6. Compute PQ ADC top-10 for each query
7. Calculate recall@10 = (intersection of PQ top-10 and true top-10) / 10, averaged over 100 queries

**Commands:**
```bash
# Create recall validation test or benchmark
cargo test test_real_embedding_recall -- --nocapture    # Print recall value
# Or create a dedicated binary in examples/
```

**Decision Tree:**
- If recall@10 >= 0.90 (M=8) → **G3 PASS**
- If recall@10 < 0.90 (M=8) → try M=16 (PQ_BENCHMARK_PLAN Section 6.2):
  - Re-train with M=16, re-encode, re-measure
  - If M=16 recall >= 0.90 → **G3 CONDITIONAL PASS** (M=16 required)
  - If M=16 recall < 0.90 → try M=32 or lower-D model
  - If all mitigations fail → **G3 FAIL**, document as experimental feature

**Expected Output:**
```
G3 Recall Validation (50K real embeddings, 768D, 100 queries):
  M=8:  recall@10 = X.XX
  M=16: recall@10 = Y.YY (if M=8 < 0.90)
  Verdict: PASS / CONDITIONAL PASS / FAIL
```

**Acceptance:**
- [ ] recall@10 measured on real embeddings (NOT synthetic uniform)
- [ ] Model documented: all-mpnet-base-v2, 768D
- [ ] 100 queries, seed=42, brute-force L2 ground truth
- [ ] Mitigation path followed if M=8 < 0.90

---

### W47.4d: Full Regression (0.5h) — TEST_ENGINEER

**Dependency:** W47.4a complete

**Commands:**
```bash
cargo test --lib --features parallel     # All tests with parallel
cargo test --lib                          # All tests without parallel
cargo clippy -- -D warnings               # Lint
cargo check --target wasm32-unknown-unknown   # WASM
```

**Acceptance:**
- [ ] 1013+ tests pass (with and without parallel feature)
- [ ] 0 clippy warnings
- [ ] WASM build succeeds

---

## Day 4 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~8.5h |
| G4 verdict | PASS or FAIL (native + WASM) |
| G3 verdict | PASS or FAIL (real embeddings) |
| All 3 gates | Definitive by EOD |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W47.4a | 3h | | |
| W47.4b | 2h | | |
| W47.4c | 3h | | |
| W47.4d | 0.5h | | |
| **Total** | **8.5h** | | |

---

## Handoff to Day 5

**Codebase state at EOD:**
- All 3 training optimizations in `product.rs` (early-stop + reduced iters + parallel)
- G2, G3, G4 all have definitive PASS/FAIL
- rayon dependency added (behind feature flag, excluded from WASM)
- WASM bundle size verified

**Day 5 prerequisites satisfied:**
- [ ] G3 recall number available (for B4 comparison)
- [ ] Same 100 query vectors and ground truth cached (for B4 reuse)
- [ ] All gate results ready for GO/NO-GO update

**Day 5 focus:** B4 PQ vs BQ+rescore comparison + GO/NO-GO update + end-of-week hostile review

---

**END OF DAY 4 TASKS**

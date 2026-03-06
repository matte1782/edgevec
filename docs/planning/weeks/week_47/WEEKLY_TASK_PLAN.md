# Week 47: PQ Validation — WASM Exports, Real-Embedding Recall, Training Optimization

**Status:** [APPROVED]
**Sprint Goal:** Resolve all CONDITIONAL GO items from W46: deliver WASM PQ exports (G2), validate recall on real embeddings (G3), optimize training to <30s native / <60s WASM at 100K (G4)
**Dates:** 2026-04-07 to 2026-04-11 (5 core days + 1 overflow)
**Prerequisites:** W46 [COMPLETE], GATE_W46_COMPLETE.md [CONDITIONAL GO], PQ_GO_NOGO_DECISION.md [REVISED]
**Milestone:** 10.4 Phase 4 (PQ Validation)

---

## Strategic Context

W46 delivered PQ core implementation with a **CONDITIONAL GO** verdict. Three gates remain open:
- **G2:** ADC latency PASS on native (37.6 ns) but UNTESTED on WASM
- **G3:** Recall INCONCLUSIVE — synthetic uniform data at 768D is meaningless (lesson #64)
- **G4:** Training FAIL — 198.7s at 100K vs <30s native / <60s WASM budget (3.3x-6.6x over)

W47 resolves all three. If ALL pass, PQ becomes a production-ready quantization method. If any FAIL, the feature is documented as experimental with known limitations.

### Key References (READ BEFORE ANY TASK)

| Document | Path | Purpose |
|:---------|:-----|:--------|
| PQ GO/NO-GO Decision | `docs/benchmarks/PQ_GO_NOGO_DECISION.md` | W46 results, gate thresholds, mitigations |
| PQ Benchmark Plan | `docs/research/PQ_BENCHMARK_PLAN.md` | Methodology, reproducibility protocol |
| PQ Implementation | `src/quantization/product.rs` | Source of truth — 7 PqError variants, 33 tests |
| Existing Benchmarks | `benches/pq_bench.rs` | B1/B2/B5/B7 native harness (reuse, don't rebuild) |
| WASM Module | `src/wasm/mod.rs` | Existing WASM patterns (handle/opaque-pointer style) |
| Architecture | `docs/architecture/ARCHITECTURE.md` | Performance budgets |
| CLAUDE.md | `.claude/CLAUDE.md` | Gate thresholds: G2 <150ns P99, G3 >0.90, G4 native <30s / WASM <60s |

### Accepted-As-Is from W46 (DO NOT RE-OPEN)

- PqSearchResult derives PartialEq but not Eq (correct for f32)
- encode_batch short-circuits on first error (documented behavior)
- compute_distance assert_eq on hot path (internal invariant, not user-facing)
- Recall near-zero on uniform random 768D data (mathematically expected)

---

## Estimation Notes

**Optimistic total:** 32h
**3x ceiling:** 96h (disaster scenario boundary — not a target)
**Planned:** ~45h across 6 days (~7.5h/day) — 1.4x multiplier on optimistic

**3x Rule Exception (documented):**
1. **WASM exports (Days 1-2):** Follow existing patterns in `src/wasm/mod.rs` — this is pattern-matching, not greenfield design. Handle/opaque-pointer style is established.
2. **Embedding generation (Day 1):** Python script with sentence-transformers is mechanical — model choice and corpus are pre-decided.
3. **Training optimization (Days 3-4):** Specific strategies enumerated in PQ_GO_NOGO_DECISION.md mitigations — implementation is guided, not exploratory.
4. **For creative/uncertain work (recall validation, WASM benchmarks):** 2x multiplier applied.

**Buffer strategy:** Day 6 is overflow. Per-task estimates include ~30% padding.

---

## Critical Path

```
Track A (Prerequisites — Day 1):
  W47.0a (git push) ─┐
  W47.1a (embeddings)─┼─► Day 2 unblocked
  W47.1b (WASM exports)┘
                        |
Track B (WASM — Days 2-3):                    Track C (Optimization — Days 3-4):
  W47.2a (WASM harness HTML)                    [INDEPENDENT of Track B — modifies
  W47.2b (WASM harness JS)                       product.rs, not wasm/mod.rs]
  W47.2c (Playwright verify)                    W47.3b (early-stop)
         |                                      W47.3c (reduce iters)
  W47.3a (G2 WASM ADC bench)                          |
         |                                      W47.4a (parallel subspaces)
  HOSTILE REVIEW #1 (Mid-Week)                  W47.4b (combined G4 — native <30s)
  [Scope: WASM only — does NOT                         |
   block Track C]                               W47.4b-wasm (G4 WASM <60s — if feasible)
         |                                             |
         └──────────────┬──────────────────────────────┘
                        |
Track D (Recall — Days 4-5):
  W47.4c (G3 recall validation) → W47.5a (B4 BQ+rescore) → W47.5b (Final GO/NO-GO update)
                                                                    |
                                                        HOSTILE REVIEW #2 (End-of-Week)
                                                                    |
                                                        Day 6 (Fix findings + Commit)
                                                        [Monday — weekend gap, extra-detailed handoff]
```

**Track Independence Note (M3 fix):** Track C (training optimization) modifies `src/quantization/product.rs`. Track B (WASM) modifies `src/wasm/mod.rs`. These are completely independent code paths. The mid-week hostile review (W47.3d) scopes ONLY to WASM exports + G2 benchmark. Track C can proceed in parallel regardless of the hostile review verdict.

**DOUBLE HOSTILE REVIEW GATES:**
1. **Mid-week (end of Day 3):** Review WASM exports + G2 WASM benchmark result
2. **End-of-week (end of Day 5):** Review G3/G4 results + updated GO/NO-GO document

**Key decision points:**
- End of Day 1: Do real embeddings exist as a loadable binary? (Prerequisite gate)
- End of Day 3: Does WASM ADC meet <150ns P99? (G2 final gate)
- End of Day 4: Does combined training optimization achieve <30s native / <60s WASM? (G4 gate)
- End of Day 5: Does recall@10 >0.90 on real embeddings? (G3 gate)

---

## Day-by-Day Summary

### Day 1 (2026-04-07): Prerequisites — Embeddings, WASM Exports, Git Push

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W47.0a | Push 6 pending commits to origin/main | PLANNER | 0.25h | Git | `git push` succeeds; `git log --oneline origin/main..HEAD` returns empty |
| W47.1a | Generate real embedding dataset with Python/sentence-transformers | BENCHMARK_SCIENTIST | 2h | File + Verify | `tests/data/embeddings_768d_50k.bin` exists; contains 50,000 vectors x 768 dims x f32 = 153,600,000 bytes; model: `all-mpnet-base-v2`; corpus: first 50K sentences from MTEB STS benchmark or English Wikipedia (fallback: any English text corpus with 50K+ sentences); includes `tests/data/generate_embeddings.py` script for reproducibility. **Verification:** `python -c "import numpy as np; d=np.fromfile('tests/data/embeddings_768d_50k.bin', dtype='float32').reshape(-1, 768); assert d.shape == (50000, 768); assert np.all(np.isfinite(d))"` passes. File added to `.gitignore` (too large for git); script is committed instead |
| W47.1b | Implement PQ WASM exports in `src/wasm/mod.rs` | WASM_SPECIALIST | 4h | Compile + Unit | Three `#[wasm_bindgen]` functions added: (1) `train_pq(data: &[f32], dims: u32, m: u32, ksub: u32, max_iters: u32) -> Result<JsValue, JsValue>` returning opaque codebook handle, (2) `encode_pq(codebook_handle: &JsValue, vector: &[f32]) -> Result<Uint8Array, JsValue>` returning M-byte PQ code, (3) `pq_search(codebook_handle: &JsValue, codes: &[u8], num_codes: u32, query: &[f32], k: u32) -> Result<JsValue, JsValue>` returning top-k results. Follow existing handle pattern from `EdgeVecIndex`. `cargo check --target wasm32-unknown-unknown` succeeds |
| W47.1c | Add PQ WASM integration tests | TEST_ENGINEER | 1.5h | Unit + WASM | Two test levels: (1) **Native-side tests** (`cargo test`): at least 3 tests verifying the WASM wrapper logic calls through to PQ correctly — `test_wasm_pq_train_returns_handle`, `test_wasm_pq_encode_returns_codes`, `test_wasm_pq_search_returns_results`. (2) **WASM-side smoke test** (`wasm-pack test --node` or via Playwright on Day 2): verify `train_pq()` callable from JS and returns non-null. Native tests pass with `cargo test`; WASM smoke test deferred to W47.2c Playwright verification |
| W47.1d | Regression: `cargo test --lib` + `cargo clippy -- -D warnings` + `cargo check --target wasm32-unknown-unknown` | TEST_ENGINEER | 0.5h | Full suite | 1013+ lib tests pass, 0 clippy warnings, WASM build succeeds |

**Day 1 exit criterion:** Real embeddings file exists and is loadable. WASM PQ exports compile and pass unit tests. 6 pending commits pushed.

### Day 2 (2026-04-08): WASM Benchmark Harness + Playwright Integration

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W47.2a | Create `tests/wasm/pq_bench.html` — WASM PQ benchmark page | BENCHMARK_SCIENTIST | 2h | Browser | HTML page loads WASM module, calls `train_pq()` with 1K synthetic vectors, displays "PQ WASM Ready" in console. Manually verified in Chrome |
| W47.2b | Create `tests/wasm/pq_bench.js` — benchmark harness JS | BENCHMARK_SCIENTIST | 2.5h | Playwright | JS harness implements: (1) `benchADC(nCodes, nQueries, iterations)` — trains codebook, encodes nCodes vectors, runs nQueries ADC scans, reports median/P99 per-candidate ns. (2) `benchTraining(nVectors, iterations)` — measures codebook training time. (3) Pre-computes brute-force ground truth for recall measurement (cost excluded from timing per PQ_BENCHMARK_PLAN Section 2.3). Playwright `browser_navigate` succeeds; `browser_evaluate` returns timing JSON |
| W47.2c | Build WASM release + verify harness via Playwright | WASM_SPECIALIST | 1.5h | Playwright | `wasm-pack build --release` succeeds; `npx serve .` serves project; Playwright navigates to `http://localhost:PORT/tests/wasm/pq_bench.html`; `browser_evaluate('benchADC(1000, 1, 1)')` returns valid timing result; Chrome version captured via `navigator.userAgent` |

**Day 2 exit criterion:** WASM benchmark harness is functional. Playwright can execute PQ benchmarks and collect results programmatically.

### Day 3 (2026-04-09): G2 WASM Benchmark + Training Optimization Start + MID-WEEK HOSTILE REVIEW

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W47.3a | Run G2: WASM ADC search latency (100K codes, 10 queries, M=8) via Playwright | BENCHMARK_SCIENTIST | 2h | Benchmark | Per-candidate latency < 150ns WASM P99 (G2 threshold). 10 measured iterations, 3 warmup. Report median and P99. Throughput counter: `Elements(10 * 100000)` per iteration (lesson #65). If P99 > 3x median, investigate per PQ_BENCHMARK_PLAN Section 1.3 item 5. Hardware + Chrome version documented |
| W47.3b | Training optimization #1: Early-stop on convergence delta | RUST_ENGINEER | 2h | Unit + Bench | Add `convergence_threshold: f32` parameter to `PqCodebook::train()` (default: 1e-4). K-means halts when centroid movement < threshold. `test_train_early_stop_converges` passes. Measure 100K training time with early-stop vs baseline (198.7s). Document improvement percentage |
| W47.3c | Training optimization #2: Reduced max iterations | RUST_ENGINEER | 1h | Bench | Measure 100K training time with max_iters=5 + early-stop. Compare vs baseline. Document: (1) training time in seconds, (2) recall impact (if any) at 10K scale |
| **W47.3d** | **MID-WEEK HOSTILE REVIEW: WASM exports + G2 benchmark** | **HOSTILE_REVIEWER** | **1.5h** | **Review** | **GO verdict on WASM PQ exports in `src/wasm/mod.rs` + G2 WASM result. 0 Critical, 0 Major. Attack vectors: (1) WASM exports follow existing handle pattern, (2) no `unwrap()` in WASM code, (3) G2 result methodology matches PQ_BENCHMARK_PLAN, (4) benchmark is reproducible, (5) error handling is complete** |

**Day 3 exit criterion:** G2 WASM result measured and documented. WASM exports hostile-reviewed. Early-stop and reduced-iters optimizations implemented with measured improvements.

**HALT CONDITION:** If mid-week hostile review returns NO-GO, Day 4 **Track B/D tasks** (G3 recall, B4 comparison) are BLOCKED until fixes resubmitted. **Track C** (training optimization W47.4a) can proceed regardless — it modifies `product.rs`, not `wasm/mod.rs`. Fix all criticals + majors, resubmit for Round 2.

### Day 4 (2026-04-10): Training Optimization Final + G3 Recall Validation

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W47.4a | Training optimization #3: Parallel subspace training (rayon) | RUST_ENGINEER | 3h | Bench + Unit | Add `rayon` dependency (behind `parallel` feature flag, gated with `#[cfg(not(target_arch = "wasm32"))]`). Train M subspaces concurrently using `par_iter`. `test_parallel_train_deterministic` — same seed produces identical codebook (parallel must not break determinism; use per-subspace seeding via `ChaCha8Rng::seed_from_u64(42 + subspace_index)`). Measure 100K training time with all optimizations combined. Document improvement. **WASM bundle size verification:** record `pkg/edgevec_bg.wasm` size before and after — increase must be < 5%. `wasm-pack build --release` must succeed |
| W47.4b | G4 final measurement: Combined optimization benchmark (native + WASM) | BENCHMARK_SCIENTIST | 2h | Benchmark | Run B7 equivalent with all optimizations (early-stop + reduced iters + parallel). **Native:** 100K training time < 30s (G4 native threshold per PQ_BENCHMARK_PLAN.md line 664). **WASM:** 100K training time < 60s (G4 WASM threshold per PQ_BENCHMARK_PLAN.md line 665). Note: WASM build excludes rayon (`cfg(not(target_arch = "wasm32"))`), so WASM measures early-stop + reduced iters only. 3 runs minimum per target, report median. If native > 30s or WASM > 60s, document and escalate. If WASM 100K crashes Chrome, reduce to 50K and extrapolate per PQ_BENCHMARK_PLAN Section 6.5 |
| W47.4c | G3: Real-embedding recall validation (50K, 768D, M=8) | BENCHMARK_SCIENTIST | 3h | Benchmark | Load `tests/data/embeddings_768d_50k.bin`. Train PQ codebook on 50K real embeddings. Encode all 50K. Run 100 queries (randomly sampled from dataset). Compute recall@10 against brute-force L2 ground truth. **recall@10 > 0.90** (G3 threshold). If < 0.90, try M=16 as mitigation per PQ_BENCHMARK_PLAN Section 6.2. Report both M=8 and M=16 results |
| W47.4d | Regression: `cargo test --lib` + `cargo clippy -- -D warnings` + `cargo check --target wasm32-unknown-unknown` | TEST_ENGINEER | 0.5h | Full suite | 1013+ lib tests pass (+ new WASM PQ tests), 0 clippy warnings, WASM build succeeds |

**Day 4 exit criterion:** G4 training optimization measured. G3 recall validated on real embeddings. All gates have definitive PASS/FAIL.

### Day 5 (2026-04-11): B4 Comparison + Final GO/NO-GO Update + END-OF-WEEK HOSTILE REVIEW

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W47.5a | B4: PQ vs BQ+rescore recall comparison on real embeddings | BENCHMARK_SCIENTIST | 2h | Benchmark | Using same 50K real embeddings and same 100 query vectors from W47.4c: (1) PQ recall@10 (reuse W47.4c result). (2) BQ+rescore recall@10: binarize all 50K vectors via `BinaryQuantizer::quantize_batch()` from `src/quantization/binary.rs`, Hamming-distance top-100 filter via `hamming_distance()` from `src/similarity.rs`, then f32 L2 rescore on original vectors to select top-10. Memory note: 50K x 768D dual-representation = ~147MB f32 + ~4.6MB BQ = ~152MB total — fits within 32GB RAM. Same brute-force L2 ground truth as W47.4c. Document: recall@10 for both methods, latency comparison, which method is better and under what conditions |
| W47.5b | Update PQ_GO_NOGO_DECISION.md with W47 validation results | PLANNER + BENCHMARK_SCIENTIST | 2h | Document | Add W47 section to `docs/benchmarks/PQ_GO_NOGO_DECISION.md`. Update all 6 gates with final PASS/FAIL (no more INCONCLUSIVE or UNTESTED). If all gates PASS: verdict = **GO**. If any gate FAIL: verdict = **NO-GO** or **CONDITIONAL** with documented limitations. Include: WASM benchmark environment, real-embedding results, training optimization measurements |
| W47.5c | LOW: PQ types in `lib.rs` top-level re-exports | RUST_ENGINEER | 0.5h | Compile | `pub use quantization::product::{PqCodebook, PqCode, DistanceTable, PqSearchResult, PqError}` in `src/lib.rs`. `cargo build` succeeds |
| W47.5d | LOW: `dimensions=0` validation in `PqCodebook::train()` | RUST_ENGINEER | 0.5h | Unit | `test_train_zero_dimensions` returns `Err(PqError::InvalidM)` or new appropriate error. No panic |
| **W47.5e** | **END-OF-WEEK HOSTILE REVIEW: G3/G4 results + updated GO/NO-GO** | **HOSTILE_REVIEWER** | **2h** | **Review** | **GO verdict on updated `docs/benchmarks/PQ_GO_NOGO_DECISION.md`. 0 Critical, 0 Major. Attack vectors: (1) G3 recall methodology uses real embeddings not synthetic, (2) G4 optimization actually achieves <60s, (3) all 6 gates have definitive verdicts, (4) no cherry-picked results, (5) training optimization doesn't degrade recall, (6) B4 comparison is fair (same data, same queries), (7) decision follows from data** |

**Day 5 exit criterion:** All 6 PQ gates have definitive PASS/FAIL. GO/NO-GO document updated with W47 results. Hostile review complete.

### Day 6 (2026-04-14, Monday): Overflow / Fix Findings + ROADMAP Update + Commit

**Weekend gap note:** Day 5 (Friday) → Day 6 (Monday) has a 2-day gap. Day 5 handoff in `DAY_5_TASKS.md` must be extra-detailed to survive context loss. Engineer should re-read Day 5 handoff notes before starting Day 6.

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W47.6a | Fix all hostile review findings (both rounds) | RUST_ENGINEER | 2h | Review fixes | All critical + major issues from both hostile reviews resolved |
| W47.6b | Update ROADMAP.md — Milestone 10.4 Phase 4 status + W47 actuals | PLANNER | 1h | Document | ROADMAP reflects actual W47 outcomes: G2/G3/G4 final verdicts, PQ feature status (GO or NO-GO with limitations) |
| W47.6c | Update CHANGELOG.md with W47 work | PLANNER | 0.5h | Document | W47 section added: WASM PQ exports, training optimization, recall validation results |
| W47.6d | Full regression: `cargo test --lib` + `cargo clippy -- -D warnings` + `cargo check --target wasm32-unknown-unknown` | TEST_ENGINEER | 0.5h | Full suite | All tests pass (1013+ existing + new W47 tests), 0 clippy warnings, WASM build succeeds |
| W47.6e | Commit all W47 work | PLANNER | 0.5h | Git | Conventional commit: `feat(w47): PQ validation — WASM exports + real-embedding recall + training optimization` |
| W47.6f | Create `.claude/GATE_W47_COMPLETE.md` | PLANNER | 0.5h | Gate | Gate file documents: both hostile review verdicts, G2/G3/G4 final results, all carry-forward items addressed |

**If surplus time available**, prioritize:
1. LOW: PQ inline doc tests (`# Examples` sections in `product.rs`)
2. LOW: `max_iters=0` doc note
3. npm publish `edgevec-langchain@0.2.0` (if user ready with OTP)

---

## Daily Execution Files

Each day has a dedicated execution file following the established project pattern (W40, W45). These are atomic — an engineer can pick up any `DAY_N_TASKS.md` and execute without re-reading this weekly plan.

| File | Content | Created |
|:-----|:--------|:--------|
| `docs/planning/weeks/week_47/DAY_1_TASKS.md` | Prerequisites: git push, embeddings, WASM exports | Before Day 1 |
| `docs/planning/weeks/week_47/DAY_2_TASKS.md` | WASM benchmark harness + Playwright integration | Before Day 2 |
| `docs/planning/weeks/week_47/DAY_3_TASKS.md` | G2 WASM benchmark + training optimization + hostile review | Before Day 3 |
| `docs/planning/weeks/week_47/DAY_4_TASKS.md` | Parallel subspaces + G4 final + G3 recall validation | Before Day 4 |
| `docs/planning/weeks/week_47/DAY_5_TASKS.md` | B4 comparison + GO/NO-GO update + hostile review | Before Day 5 |
| `docs/planning/weeks/week_47/DAY_6_TASKS.md` | Overflow + fixes + ROADMAP + commit (extra-detailed handoff for Monday) | Before Day 6 |

Each daily file contains:
1. **Day objective** and success criteria
2. **Pre-task context loading checklist** (which files to read)
3. **Task sequence** with dependencies clearly marked
4. **Specific command sequences** (cargo, Python, Playwright commands)
5. **Expected output format** for each task
6. **Decision trees** for conditional paths (e.g., if G3 < 0.90 → mitigation)
7. **Handoff notes** (codebase state at EOD, next day prereqs)
8. **Time tracking template** (estimated vs actual per task)

---

## Deliverables

| Deliverable | Target Day | Owner |
|:------------|:-----------|:------|
| `DAY_1_TASKS.md` through `DAY_6_TASKS.md` — Daily execution files | Pre-sprint | PLANNER |
| `tests/data/embeddings_768d_50k.bin` — Real embedding dataset | Day 1 | BENCHMARK_SCIENTIST |
| `tests/data/generate_embeddings.py` — Reproducible generation script | Day 1 | BENCHMARK_SCIENTIST |
| WASM PQ exports in `src/wasm/mod.rs` (train_pq, encode_pq, pq_search) | Day 1 | WASM_SPECIALIST |
| `tests/wasm/pq_bench.html` + `tests/wasm/pq_bench.js` — Playwright-automated WASM harness | Day 2 | BENCHMARK_SCIENTIST |
| G2 WASM ADC benchmark result | Day 3 | BENCHMARK_SCIENTIST |
| Training optimizations (early-stop, reduced iters, parallel subspaces) in `product.rs` | Days 3-4 | RUST_ENGINEER |
| G4 combined optimization benchmark result | Day 4 | BENCHMARK_SCIENTIST |
| G3 real-embedding recall result | Day 4 | BENCHMARK_SCIENTIST |
| B4 PQ vs BQ+rescore comparison | Day 5 | BENCHMARK_SCIENTIST |
| Updated `docs/benchmarks/PQ_GO_NOGO_DECISION.md` with all 6 gates final | Day 5 | PLANNER + BENCHMARK_SCIENTIST |
| Mid-week hostile review report | Day 3 | HOSTILE_REVIEWER |
| End-of-week hostile review report | Day 5 | HOSTILE_REVIEWER |
| Updated `docs/planning/ROADMAP.md` | Day 6 | PLANNER |
| Updated `CHANGELOG.md` | Day 6 | PLANNER |
| `.claude/GATE_W47_COMPLETE.md` | Day 6 | PLANNER |

---

## Carry-Forward Traceability

Every HIGH carry-forward item from GATE_W46_COMPLETE.md is mapped to a task:

| W46 Carry-Forward | W47 Task ID | Status |
|:-------------------|:------------|:-------|
| WASM PQ exports in `src/wasm/mod.rs` | W47.1b | PLANNED |
| WASM benchmark harness | W47.2a, W47.2b | PLANNED |
| G2 WASM ADC benchmark | W47.3a | PLANNED |
| G3 Real-embedding recall validation | W47.4c | PLANNED |
| G4 Training optimization | W47.3b, W47.3c, W47.4a, W47.4b | PLANNED |
| BQ+rescore comparison (MEDIUM) | W47.5a | PLANNED |
| PQ types in lib.rs re-exports (LOW) | W47.5c | PLANNED |
| dimensions=0 validation (LOW) | W47.5d | PLANNED |
| PQ inline doc tests (LOW) | Day 6 surplus | PLANNED |
| max_iters=0 doc note (LOW) | Day 6 surplus | PLANNED |

**No items silently dropped.** All 10 carry-forward items are accounted for.

---

## Double Hostile Review Protocol

### Review #1: Mid-Week WASM + G2 Review (Day 3)

**Scope:** `src/wasm/mod.rs` PQ exports + G2 WASM benchmark result
**Attack vectors:**
1. **Pattern compliance:** WASM exports follow existing handle/opaque-pointer pattern
2. **Safety:** No `unwrap()` in WASM code. All `Result` types propagated via `JsValue`
3. **G2 methodology:** Benchmark follows PQ_BENCHMARK_PLAN reproducibility protocol exactly
4. **Reproducibility:** Chrome version documented, 10 measured iterations, warmup performed
5. **Error handling:** All three WASM functions return `Result`, not panic
6. **Regression:** WASM build still succeeds, existing tests unbroken

**Verdict required before Day 4 proceeds.**

### Review #2: End-of-Week Validation Review (Day 5)

**Scope:** Updated `docs/benchmarks/PQ_GO_NOGO_DECISION.md` + G3/G4 results
**Attack vectors:**
1. **Real data:** G3 uses real embeddings (all-mpnet-base-v2, 768D), NOT synthetic uniform
2. **G4 achievement:** Combined optimization actually < 60s (not just one strategy in isolation)
3. **Gate integrity:** All 6 gates (G1-G6) have definitive PASS/FAIL with specific numbers
4. **No cherry-picking:** Negative results (if any) are prominently documented
5. **Recall impact:** Training optimization doesn't degrade recall (compared before/after)
6. **B4 fairness:** PQ vs BQ comparison uses identical query set and ground truth
7. **Decision logic:** Final verdict follows logically from all 6 gate results

**Verdict required before Day 6 commit.**

---

## Sprint-Level Acceptance Criteria

| Criterion | Pass/Fail |
|:----------|:----------|
| WASM PQ exports compile and pass unit tests (train_pq, encode_pq, pq_search) | [ ] |
| G2: WASM ADC latency < 150ns/candidate P99 | [ ] |
| G3: recall@10 > 0.90 on real embeddings (768D, 50K, M=8 or M=16) | [ ] |
| G4: 100K training native < 30s AND WASM < 60s with combined optimizations | [ ] |
| B4: PQ vs BQ+rescore comparison documented on real data | [ ] |
| PQ_GO_NOGO_DECISION.md updated — all 6 gates definitive | [ ] |
| No existing tests broken (1013+ lib pass) | [ ] |
| Clippy clean (`-D warnings`) | [ ] |
| WASM build: `cargo check --target wasm32-unknown-unknown` succeeds | [ ] |
| Mid-week hostile review: GO (0 Critical, 0 Major) | [ ] |
| End-of-week hostile review: GO (0 Critical, 0 Major) | [ ] |
| ROADMAP updated with W47 actuals | [ ] |
| GATE_W47_COMPLETE.md created | [ ] |
| 6 pending commits pushed to origin | [ ] |

---

## Risk Register

| Risk | Prob | Impact | Mitigation |
|:-----|:-----|:-------|:-----------|
| sentence-transformers install fails on Windows | LOW | HIGH | Use WSL2 or pre-built Docker image; fallback: download pre-embedded dataset from HuggingFace Hub |
| Recall@10 < 0.90 on real embeddings (G3 fail) | MEDIUM | HIGH | Try M=16 (doubles subquantizers). If still <0.90, try M=32 or lower dimensionality model (all-MiniLM-L6-v2 at 384D). Document as experimental if still fails |
| G4 training still > 60s after all optimizations | MEDIUM | MEDIUM | Accept degraded threshold with documentation. Consider mini-batch k-means as additional optimization. Mark training as "offline recommended for N>50K" |
| WASM ADC > 150ns P99 (G2 fail) | LOW | MEDIUM | Native headroom is 4x (37.6ns). If WASM overhead > 4x, investigate V8 deoptimization. Try `wasm-opt -O3` post-processing |
| Chrome tab crashes during 100K WASM training | LOW | MEDIUM | Follow PQ_BENCHMARK_PLAN Section 6.5: reduce to 50K, extrapolate, document Web Worker requirement |
| Playwright MCP unavailable or fails to connect | MEDIUM | MEDIUM | **Pre-Day-2 check:** verify Playwright MCP plugin is installed and `browser_navigate` succeeds on a test URL. If unavailable: fallback to manual Chrome profiling with DevTools Performance tab + `performance.now()` timing in console. Document manual reproduction steps. If neither works: use Node.js WASM benchmarks via `wasm-pack test --node` with `performance.now()` (lower fidelity than Chrome but still valid) |
| Mid-week hostile review returns NO-GO | LOW | HIGH | Fix all findings Day 3 evening, resubmit Day 4 morning. If Round 2 also NO-GO, escalate to user |
| Parallel subspace training breaks determinism | MEDIUM | MEDIUM | Use per-subspace seeded RNG (ChaCha8Rng::seed_from_u64(42 + subspace_index)). Test determinism explicitly |
| rayon dependency increases WASM bundle size | LOW | LOW | Use `#[cfg(not(target_arch = "wasm32"))]` to exclude rayon from WASM build. WASM stays single-threaded |
| embeddings_768d_50k.bin too large for git | HIGH | LOW | Add to `.gitignore`. Include `generate_embeddings.py` script instead. CI can regenerate |

---

## Dependencies

| This Week Depends On | What |
|:---------------------|:-----|
| W46 PQ implementation [COMPLETE] | `src/quantization/product.rs` with 33 tests |
| W46 native benchmarks [COMPLETE] | `benches/pq_bench.rs` (B1/B2/B5/B7) |
| PQ_GO_NOGO_DECISION.md [REVISED] | Gate thresholds and W46 baseline numbers |
| PQ_BENCHMARK_PLAN.md [REVISED] | Methodology, reproducibility protocol |
| Existing WASM patterns | `src/wasm/mod.rs` handle/opaque-pointer style |
| Python + sentence-transformers | For embedding generation (Day 1 prerequisite) |
| Playwright MCP | For WASM benchmark automation (Day 2-3) |

| This Week Blocks | What |
|:-----------------|:-----|
| v0.10.0 release | PQ validation is last item for v0.10.0 |
| v1.0 planning (Week 48+) | Needs final feature set confirmed |
| PQ production documentation | Only if GO verdict |

---

## NOT IN SCOPE THIS WEEK

| Task | Why Deferred |
|:-----|:-------------|
| PQ persistence format | Phase 5+ if GO — needs design RFC |
| PQ + HNSW integration (non-exhaustive search) | Phase 5+ if GO — architectural change |
| PqConfig builder pattern | v0.10.0 polish, not validation |
| npm publish `edgevec-langchain@0.2.0` | User handles OTP independently (surplus time only) |
| BM25 integration | Zero community requests |
| WASM training benchmark (B5/B7 WASM) | Training is too slow for in-browser use at 100K; focus on native optimization. WASM training deferred until parallel Web Workers are available |

---

## Anti-Error Checklist (Self-Validation)

- [x] Every HIGH carry-forward item has a corresponding task ID (see Traceability table)
- [x] No task > 16 hours (largest: W47.1b at 4h)
- [x] All acceptance criteria are binary pass/fail with specific numbers
- [x] Dependencies reference specific files/tests, not vague descriptions
- [x] G2 threshold: <150ns/candidate WASM P99 (source: CLAUDE.md line 367)
- [x] G3 threshold: recall@10 >0.90 on real embeddings (source: CLAUDE.md line 369)
- [x] G4 threshold: native <30s / WASM <60s for 100K training (source: PQ_BENCHMARK_PLAN.md Section 4.9, lines 664-665)
- [x] Real embedding data acquisition is an EXPLICIT prerequisite task (W47.1a, Day 1)
- [x] WASM exports list 3 specific functions (train_pq, encode_pq, pq_search)
- [x] Training optimization has separate sub-tasks with individual measurements (W47.3b, W47.3c, W47.4a, W47.4b)
- [x] PqError variant count is 8 (7 original + InvalidConvergenceThreshold added W47 Day 3)
- [x] No silently dropped items — all 10 carry-forward items mapped (see Traceability)
- [x] Git push timing addressed (W47.0a, Day 1)
- [x] Existing benches/pq_bench.rs referenced (Key References table)
- [x] WASM compilation check in regression suite (W47.1d, W47.4d, W47.6d)
- [x] Commit convention: `feat(w47):` style specified (W47.6e)
- [x] DAY_1_TASKS.md through DAY_6_TASKS.md listed as deliverables (C1 fix)
- [x] WASM bundle size verification in W47.4a acceptance criteria (M4 fix)
- [x] G4 dual threshold: native <30s AND WASM <60s (M2 fix)
- [x] Track C independence from hostile review explicitly documented (M3 fix)
- [x] Weekend gap Day 5→Day 6 documented with extra-detailed handoff (m1 fix)
- [x] Playwright MCP pre-Day-2 availability check in risk register (m5 fix)

---

## HOSTILE REVIEW REQUIRED

**Before optimization/recall work begins (end of Day 3):**
- [ ] HOSTILE_REVIEWER has approved WASM PQ exports + G2 benchmark

**Before committing (end of Day 5):**
- [ ] HOSTILE_REVIEWER has approved updated GO/NO-GO decision document

---

## APPROVALS

| Role | Name | Signature | Date |
|:-----|:-----|:----------|:-----|
| PLANNER | | [APPROVED] | 2026-03-05 |
| HOSTILE_REVIEWER | | Round 1: REJECTED (1C, 4M, 5m) — all fixed | 2026-03-05 |
| HOSTILE_REVIEWER | | Round 2: CONDITIONAL GO (0C, 0M, 2m — both fixed) | 2026-03-05 |

---

**END OF WEEKLY TASK PLAN**

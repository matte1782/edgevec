# Hostile Review: W46 Days 4-5 — PQ Benchmarks + GO/NO-GO Decision

**Date:** 2026-03-28
**Reviewer:** HOSTILE_REVIEWER
**Artifact:** PQ_GO_NOGO_DECISION.md + benches/pq_bench.rs
**Author:** BENCHMARK_SCIENTIST
**Type:** Benchmark Report + Code

---

## Round 1 Verdict: REJECT (1C/5M/4m)

### Critical
- **[C1] WASM benchmarks absent.** G2/G4 gates defined against WASM targets. All results native-only.

### Major
- **[M1]** Hardware/software environment incomplete (CPU, RAM, Rust version missing)
- **[M2]** Query count reduced from 1,000 to 50/20 without justification
- **[M3]** P99 latency not reported per PQ_BENCHMARK_PLAN Section 1.3
- **[M4]** B2 Criterion throughput counter wrong by 10x (100K vs 1M elements)
- **[M5]** Super-linear scaling (3.07x for 2x data) unexplained

### Minor
- **[m1]** B6 "PQ/F32" column header misleading (values are F32/PQ compression ratio)
- **[m2]** B4 should explicitly state "deferred" not just "not meaningful"
- **[m3]** `compute_distance` uses `assert_eq!` (panic) on hot path — pre-existing
- **[m4]** No reference to raw Criterion output files

---

## Round 2 Verdict: CONDITIONAL GO

All Round 1 findings addressed in [REVISED] version:

| Finding | Resolution | Status |
|:--------|:-----------|:-------|
| C1 | WASM caveat section + "PASS (native)/UNTESTED (WASM)" + W47 action items | FIXED |
| M1 | Full environment table (CPU, RAM, Rust, commit) | FIXED |
| M2 | Justification added (results 0.02/0.00, not borderline) | FIXED |
| M3 | Range values from Criterion added for B1/B2 | ACCEPTED |
| M4 | Throughput counter fixed in pq_bench.rs | FIXED |
| M5 | Cache pressure explanation added (294MB > 36MB L3) | FIXED |
| m1 | Column header fixed to "F32/PQ (compression)" | FIXED |
| m2 | B4 explicitly states "deferred to W47" | FIXED |
| m3 | Pre-existing, not benchmark issue | ACCEPTED AS-IS |
| m4 | Criterion output in target/criterion/ | ACCEPTED AS-IS |

### Remaining Concern (Minor, non-blocking)

P99 values not available for B5/B7 (single wall-clock runs, not Criterion). Accepted because training time is the dominant cost and the single measurements are consistent with recall-test cross-validation (50K: 64.67s vs 63.2s at 25-iter scaled to 15-iter).

### Verification

| Check | Result |
|:------|:-------|
| `cargo test --lib` | 1013 passed, 0 failed |
| `cargo clippy -- -D warnings` | 0 warnings |
| Memory math independently verified | Correct |
| Gate criteria match PQ_BENCHMARK_PLAN | Verified (with WASM caveat) |

---

**HOSTILE_REVIEWER: CONDITIONAL GO — Proceed to commit.**

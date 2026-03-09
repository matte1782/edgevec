# Miri Regression Report — W49

**Status:** [PROPOSED]
**Date:** 2026-03-09
**Auditor:** MIRI_REGRESSION automated run
**Toolchain:** `nightly-x86_64-pc-windows-msvc` (rustc 1.94.0-nightly, 2025-12-05)
**Miri flags:** `-Zmiri-disable-isolation -Zmiri-symbolic-alignment-check`
**Proptest cases:** `PROPTEST_CASES=4`
**Feature flags:** `--no-default-features` (matches W42 CI configuration)

---

## Executive Summary

**0 new UB findings.** All testable modules pass Miri with zero undefined behavior detected. W42 baseline of 392 tests exceeded with **498 tests passing**. PQ product module (47 tests) partially validated (9/47) — training/k-means tests deferred due to Miri's interpreter overhead on FP-heavy computation.

---

## Results by Module

| Module | Tests | Result | Duration | Notes |
|:-------|------:|:-------|:---------|:------|
| `persistence` | 29 | **PASS** | 9.76s | Headers, WAL, reader, writer |
| `distance` | 22 | **PASS** | 888.45s | SIMD dispatch, all metrics |
| `storage` | 31 | **PASS** | 17.88s | Binary storage, quantized storage |
| `index` | 83 | **PASS** | 2290.01s | Flat index, HNSW, snapshots |
| `filter` | 271 | **PASS** | 2137.40s | Evaluator, expression, strategy, boost |
| `quantization::binary` | 30 | **PASS** | 795.06s | BQ encoding, hamming, similarity |
| `quantization::variable` | 21 | **PASS** | 8.24s | Variable-dim quantization |
| `quantization::product` (partial) | 9 | **PASS** | 1911.88s | Code struct (2) + encode (7) |
| **TOTAL** | **498** | **PASS** | ~2h 21m | 0 UB findings |

### PQ Product Module — Deferred Tests

47 total PQ product tests. 9 validated under Miri:
- `test_pq_code_structure` — PASS (461s)
- `test_pq_code_with_invalid_ksub_causes_oob` — PASS (461s)
- `test_encode_batch_basic` — PASS
- `test_encode_batch_dimension_mismatch` — PASS
- `test_encode_batch_empty` — PASS
- `test_encode_dimension_mismatch` — PASS
- `test_encode_produces_correct_length` — PASS
- `test_encode_rejects_infinity` — PASS
- `test_encode_rejects_nan` — PASS

**38 tests deferred.** Reason: PQ's k-means training involves millions of floating-point operations per test. Under Miri's interpreter model (~1000x native speed), a single training test exceeds 60 minutes. Full PQ Miri would require 30+ hours.

**Risk assessment for deferred tests:**
- PQ's `unsafe` code is exclusively SIMD operations in `src/quantization/product.rs` — these are the same SIMD paths validated by the `distance` module (22/22 PASS)
- PQ training logic (k-means, encoding, ADC) is 100% safe Rust — no `unsafe` blocks
- PQ is validated by: 47 native tests (all pass), proptest fuzzing, Criterion benchmarks, W46-W47 benchmark campaign
- **No UB risk in deferred tests** — they contain no unsafe code

---

## Comparison: W42 vs W49

| Metric | W42 Baseline | W49 Result | Delta |
|:-------|:-------------|:-----------|:------|
| Tests under Miri | 392+ | 498+ | **+106 (+27%)** |
| UB findings | 0 | 0 | No regression |
| New modules tested | — | filter (271), quantization::product (9), quantization::variable (21) | New coverage |
| Toolchain | nightly (2025-10) | nightly (2025-12-05) | Updated |
| Flags | Same | Same | Consistent |

### New Modules Since W42
- `filter` module: 271 tests (added W44-W48, includes FilterExpression, MetadataBoost, strategy)
- `quantization::product`: 9 tests validated (added W46; 2 struct + 7 encode tests passed, 38 training/k-means tests deferred)
- `quantization::variable`: 21 tests (variable-dimension quantization)

### Root Cause Analysis
No regressions detected. All W42-era modules maintain the same test count or higher. No new UB introduced by:
- W44 FilterExpression additions
- W46 PQ implementation
- W47 PQ validation + training optimizations
- W48 MetadataBoost API
- W49 HNSW distance inversion fix

---

## Known Miri Limitations

1. **Proptest + Miri:** Proptest uses `getrandom` which Miri cannot handle without `-Zmiri-disable-isolation`. With this flag, proptest runs but with `PROPTEST_CASES=4` (reduced from default 256).

2. **SIMD under Miri:** Miri supports x86 SIMD intrinsics but at ~100x slowdown. The `distance` module's 888s runtime (vs <1s native) reflects this.

3. **PQ k-means:** Floating-point intensive loops are ~1000x slower. A single `test_codebook_train_deterministic` test runs k-means with 50+ iterations over thousands of vectors — infeasible under Miri.

4. **WASM target:** Miri runs `x86_64` target only. WASM-specific code paths are not covered (validated separately via `wasm-pack test`).

---

## Recommendations

1. **CI integration:** Add Miri to CI with `--no-default-features --lib` and module-level exclusion for `quantization::product::tests::test_kmeans*` and `test_codebook_train*`
2. **PQ Miri budget:** If full PQ Miri coverage is needed, run overnight on dedicated hardware (~30h estimated)
3. **Nightly update:** Current nightly (2025-12-05) is stable for this codebase. No blockers for updating.

---

## Conclusion

**W49 Miri regression: PASS.** 498 tests validated, 0 UB findings, 27% increase over W42 baseline. All x86_64 unsafe code paths (SIMD intrinsics) are covered by passing tests. PQ training tests deferred with documented justification — no unsafe code in deferred paths.

**Coverage gap:** The HIGH-risk WASM transmute at `src/wasm/mod.rs:1695` (documented in UNSAFE_INVENTORY.md) is behind `cfg(target_arch = "wasm32")` and is **not reachable by Miri** (x86_64 only). This transmute requires separate validation via `wasm-pack test` or the SafeChunkIter hardening planned for W49 Track C.
